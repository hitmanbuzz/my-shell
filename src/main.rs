use std::{
    fs::OpenOptions,
    io::{self, Write},
    process::{Child, Command, Stdio, exit},
};

const MY_SHELL: &str = "my-shell";

#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Copy, PartialEq)]
enum Operator {
    PIPE,
    SEMI_COLON,
    AND,
    NULL,
}

#[derive(Debug)]
struct Cmd<'a> {
    args: Vec<&'a str>,
    op: Operator,
    redirect_out: Option<(&'a str, bool)>,
}

impl<'a> Cmd<'a> {
    fn new(args: Vec<&'a str>, op: Operator, redirect_out: Option<(&'a str, bool)>) -> Self {
        Self {
            args,
            op,
            redirect_out,
        }
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

    // private method (if use as a library)
    fn scan_tokens(&mut self, tokens: Vec<&'a str>) {
        let mut cmds = Vec::new();
        let mut current_args = Vec::new();
        let mut redirect_out = None;
        let mut i = 0;

        while i < tokens.len() {
            let token = tokens[i];
            match token {
                "|" | "&&" | ";" => {
                    let op = match token {
                        "|" => Operator::PIPE,
                        "&&" => Operator::AND,
                        ";" => Operator::SEMI_COLON,
                        _ => unreachable!(),
                    };

                    if !current_args.is_empty() {
                        cmds.push(Cmd::new(current_args.clone(), op, redirect_out));
                        current_args.clear();
                        redirect_out = None;
                    }
                }
                ">" | ">>" => {
                    let is_append = token == ">>";
                    if i + 1 < tokens.len() {
                        redirect_out = Some((tokens[i + 1], is_append));
                        i += 1; // skip the filename token
                    } else {
                        eprintln!("{}: expected file after {}", MY_SHELL, token);
                    }
                }
                _ => {
                    current_args.push(token);
                }
            }
            i += 1;
        }

        if !current_args.is_empty() {
            cmds.push(Cmd {
                args: current_args,
                op: Operator::NULL,
                redirect_out,
            });
        }

        self.cmds = cmds;
    }

    fn exec_cmds(&self) {
        let mut prev_pip = None;
        let mut child_processes: Vec<Child> = Vec::new();
        let mut skip_next = false;

        for cmd in &self.cmds {
            if cmd.args.is_empty() {
                continue;
            }

            if skip_next {
                if cmd.op == Operator::SEMI_COLON {
                    skip_next = false;
                }
                continue;
            }

            // kinda fuckup that exit is a built-in shell
            if cmd.args[0] == "exit" {
                if cmd.args.len() > 1 {
                    if let Ok(code) = cmd.args[1].parse::<i32>() {
                        exit(code);
                    } else {
                        eprintln!(
                            "{}: exit: {}: numeric argument required",
                            MY_SHELL, cmd.args[1]
                        );
                        exit(255);
                    }
                }
                exit(0);
            }

            let mut cmd_exec = Command::new(cmd.args[0]);
            if cmd.args.len() > 1 {
                cmd_exec.args(&cmd.args[1..]);
            }

            // handle >>
            let mut file_out = None;
            if let Some((file_name, append)) = cmd.redirect_out {
                match OpenOptions::new()
                    .create(true)
                    .write(true)
                    .append(append)
                    .truncate(!append)
                    .open(file_name)
                {
                    Ok(file) => file_out = Some(file),
                    Err(e) => {
                        eprintln!("{}: {}: {}", MY_SHELL, file_name, e);
                        if cmd.op == Operator::AND {
                            skip_next = true;
                        }
                        continue;
                    }
                }
            }

            if let Some(pipe) = prev_pip.take() {
                cmd_exec.stdin(Stdio::from(pipe));
            }

            if let Some(f) = file_out {
                cmd_exec.stdout(Stdio::from(f));
            } else if cmd.op == Operator::PIPE {
                cmd_exec.stdout(Stdio::piped());
            } else {
                cmd_exec.stdout(Stdio::inherit());
            }

            match cmd_exec.spawn() {
                Ok(mut child) => {
                    if cmd.op == Operator::PIPE {
                        prev_pip = child.stdout.take();
                        child_processes.push(child);
                    } else {
                        let status = child.wait();
                        for mut p in child_processes.drain(..) {
                            let _ = p.wait();
                        }

                        let success = status.map(|s| s.success()).unwrap_or(false);

                        match cmd.op {
                            Operator::AND => {
                                if !success {
                                    skip_next = true;
                                }
                            }
                            Operator::SEMI_COLON | Operator::NULL => {
                                skip_next = false;
                            }
                            Operator::PIPE => unreachable!(),
                        }
                    }
                }
                Err(_) => {
                    eprintln!("{}: command not found: {}", MY_SHELL, cmd.args[0]);
                    for mut p in child_processes.drain(..) {
                        let _ = p.wait();
                    }
                    if cmd.op == Operator::AND {
                        skip_next = true;
                    }
                }
            }
        }
    }
}

fn main() {
    Command::new("clear").status().unwrap();

    println!("[WELCOME TO MY SHELL]");
    loop {
        print!("❯ ");
        io::stdout().flush().unwrap();

        let mut input = String::new();

        match io::stdin().read_line(&mut input) {
            Ok(_) => {}
            Err(e) => {
                eprintln!("failed to read input: {}", e);
                break;
            }
        }

        let mut list_cmd = ListCmd::new(&input);
        list_cmd.read_input();
        list_cmd.exec_cmds();
        println!("")
    }
}
