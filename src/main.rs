use std::{
    ffi::OsStr,
    io::{self, ErrorKind, Write},
    process::{Command, exit},
};

#[derive(Debug)]
struct Cmd<'a> {
    main: &'a str,
    args: Option<Vec<&'a str>>,
}

impl<'a> Cmd<'a> {
    fn new(main: &'a str, args: Option<Vec<&'a str>>) -> Self {
        Self { main, args }
    }
}

struct ListCmd<'a> {
    input: &'a str,
    cmds: Vec<Cmd<'a>>,
}

impl<'a> ListCmd<'a> {
    fn new(input: &'a str) -> Self {
        Self {
            input,
            cmds: Vec::new(),
        }
    }

    fn read_input(&mut self) {
        let tokens: Vec<&str> = self.input.split_whitespace().collect();
        if tokens.is_empty() {
            return;
        }

        self.scan_tokens(tokens);
    }

    fn exec_cmds(&self) {
        for cmd in self.cmds.iter() {
            let mut cmd_exec = Command::new(&cmd.main);
            if let Some(ref args) = cmd.args {
                cmd_exec.args(args);
            }

            if cmd.main == "exit" {
                let args: Vec<&OsStr> = cmd_exec.get_args().collect();
                if let Some(arg) = args.get(0) {
                    if let Some(code) = arg.to_str() {
                        let Ok(exit_code) = code.parse::<i32>() else {
                            eprintln!("my-shell: exit: {}: numeric argument required", code);
                            return;
                        };
                        exit(exit_code);
                    }
                } else {
                    exit(0);
                }
            }

            match cmd_exec.output() {
                Ok(output) => {
                    if output.status.success() {
                        match String::from_utf8(output.stdout) {
                            Ok(o) => {
                                print!("{}", o);
                                io::stdout().flush().unwrap();
                            }
                            Err(e) => {
                                eprintln!(
                                    "failed to convert to string, cmd: {:?} | error: {}",
                                    cmd, e
                                )
                            }
                        }
                    } else {
                        match String::from_utf8(output.stderr) {
                            Ok(o) => {
                                eprint!("{}", o);
                            }
                            Err(e) => {
                                eprintln!(
                                    "failed to convert to string, cmd: {:?} | error: {}",
                                    cmd, e
                                )
                            }
                        }
                    }
                }
                Err(e) => match e.kind() {
                    ErrorKind::NotFound => eprintln!("my-shell: {}: command not found", cmd.main),
                    _ => eprintln!("my-shell: command: {} | error: {}", cmd.main, e),
                },
            }
        }
    }

    // private method (if use as a library)
    fn scan_tokens(&mut self, tokens: Vec<&'a str>) {
        let mut cmd_tokens: Vec<Cmd> = Vec::new();
        let mut cmd = Cmd::new(tokens[0], None);

        if tokens.len() > 1 {
            let args: Vec<&str> = tokens.get(1..).unwrap().iter().map(|x| *x).collect();
            cmd.args = Some(args)
        }

        cmd_tokens.push(cmd);
        self.cmds = cmd_tokens;
        // return cmd_tokens;
    }
}

fn main() {
    if cfg!(target_os = "windows") {
        Command::new("cls").status().unwrap();
    } else {
        Command::new("clear").status().unwrap();
    }

    println!("[WELCOME TO MY SHELL]");
    loop {
        print!("❯ ");
        io::stdout().flush().unwrap();

        let mut input = String::new();

        match io::stdin().read_line(&mut input) {
            Ok(_) => {}
            Err(e) => eprintln!("failed to read input: {}", e),
        }

        let mut list_cmd = ListCmd::new(&input);
        list_cmd.read_input();
        list_cmd.exec_cmds();
        println!("")
    }
}
