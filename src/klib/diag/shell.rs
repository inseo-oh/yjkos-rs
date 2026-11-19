use crate::{klib, print, println};
use alloc::{
    boxed::Box,
    string::{String, ToString},
    vec::Vec,
};

struct PrintCommand;
impl<HW: klib::hal::Hardware> Command<HW> for PrintCommand {
    fn name(&self) -> &'static str {
        "print"
    }
    fn help(&self) -> &'static str {
        "i am print command"
    }
    fn parameter(
        &self,
        _arguments: &[String],
        argument_index: usize,
    ) -> Option<Box<dyn Completable>> {
        match argument_index {
            0 => Some(KeywordParameter::new(&[
                Keyword::new("world", "world keyword"),
                Keyword::new("bye", "bye keyword"),
            ])),
            _ => None,
        }
    }

    fn execute(&self, _arguments: &[String], _kglobals: *mut klib::KGlobals<HW>) {
        println!("hello, world!");
    }
}

struct TestCommand;
impl<HW: klib::hal::Hardware> Command<HW> for TestCommand {
    fn name(&self) -> &'static str {
        "test"
    }
    fn help(&self) -> &'static str {
        "run a diagnostic test"
    }
    fn parameter(
        &self,
        _arguments: &[String],
        argument_index: usize,
    ) -> Option<Box<dyn Completable>> {
        match argument_index {
            0 => Some(KeywordParameter::new(&[Keyword::new(
                "thread",
                "run the multithreading stress test",
            )])),
            _ => None,
        }
    }

    fn execute(&self, arguments: &[String], kglobals: *mut klib::KGlobals<HW>) {
        if arguments.first().is_some_and(|arg| arg == "thread") {
            // SAFETY: Shell retains this pointer to the live kernel globals.
            unsafe { klib::diag::thread::run(kglobals, klib::diag::thread::DEFAULT_DURATION_SECS) }
        } else {
            println!("usage: test thread");
        }
    }
}

fn commands<HW: klib::hal::Hardware>() -> Vec<Box<dyn Command<HW>>> {
    let mut commands: Vec<Box<dyn Command<HW>>> = Vec::new();
    commands.push(Box::new(PrintCommand));
    commands.push(Box::new(TestCommand));
    commands
}

////////////////////////////////////////////////////////////////////////////////

#[derive(Clone, Copy)]
struct Keyword {
    name: &'static str,
    help: &'static str,
}
impl Keyword {
    pub fn new(name: &'static str, help: &'static str) -> Self {
        Self { name, help }
    }
}
struct KeywordParameter {
    keywords: Vec<Keyword>,
}
impl KeywordParameter {
    pub fn new(keywords: &[Keyword]) -> Box<Self> {
        Box::new(Self {
            keywords: keywords.to_vec(),
        })
    }
}
impl Completable for KeywordParameter {
    fn choices(&self) -> Vec<Suggestion> {
        self.keywords
            .iter()
            .map(|s| Suggestion {
                value: s.name.to_string(),
                help: s.help,
            })
            .collect()
    }
}

////////////////////////////////////////////////////////////////////////////////

trait Command<HW: klib::hal::Hardware> {
    fn name(&self) -> &'static str;
    fn help(&self) -> &'static str;
    fn parameter(
        &self,
        arguments: &[String],
        argument_index: usize,
    ) -> Option<Box<dyn Completable>>;
    fn execute(&self, arguments: &[String], kglobals: *mut klib::KGlobals<HW>);
}

struct CommandNameCompletion<'a, HW: klib::hal::Hardware>(&'a [Box<dyn Command<HW>>]);
impl<HW: klib::hal::Hardware> Completable for CommandNameCompletion<'_, HW> {
    fn choices(&self) -> Vec<Suggestion> {
        self.0
            .iter()
            .map(|c| Suggestion {
                value: c.name().to_string(),
                help: c.help(),
            })
            .collect()
    }
}

#[derive(Clone)]
struct Suggestion {
    value: String,
    help: &'static str,
}
trait Completable {
    fn choices(&self) -> Vec<Suggestion>;
    fn candidates_for(&self, input: &str) -> Vec<Suggestion> {
        self.choices()
            .iter()
            .filter(|&s| s.value.starts_with(input))
            .cloned()
            .collect()
    }
    fn try_autocomplete(&self, input: &str) -> Result<Suggestion, &'static str> {
        let cand = self.candidates_for(input);
        if cand.is_empty() {
            Err("invalid command name or arguments")
        } else if cand.len() != 1 {
            Err("ambiguous command name or arguments")
        } else {
            Ok(cand[0].clone())
        }
    }
}

pub struct Shell<HW: klib::hal::Hardware> {
    command_buf: String,
    commands: Vec<Box<dyn Command<HW>>>,
}

impl<HW: klib::hal::Hardware> Shell<HW> {
    pub fn new() -> Self {
        Self {
            command_buf: String::new(),
            commands: commands(),
        }
    }
    fn command_components(&self) -> Vec<&str> {
        self.command_buf.split(" ").collect()
    }
    fn candidates(&self) -> Vec<Suggestion> {
        let components = self.command_components();
        if components.len() < 2 {
            // Return command suggestions
            return CommandNameCompletion(&self.commands).candidates_for(components[0]);
        }
        let cmd_sugg = if let Ok(name) =
            CommandNameCompletion(&self.commands).try_autocomplete(components[0])
        {
            name
        } else {
            return Vec::new();
        };
        let cmd = self
            .commands
            .iter()
            .find(|c| c.name() == cmd_sugg.value)
            .unwrap();
        let mut args = Vec::new();
        // Autocomplete command except the last component
        let mut param_idx = 0;
        while param_idx + 1 < components.len() - 1 {
            let param = if let Some(arg) = cmd.parameter(&args, param_idx) {
                arg
            } else {
                break;
            };
            args.push(
                if let Ok(arg_sugg) = param.try_autocomplete(components[param_idx + 1]) {
                    arg_sugg.value
                } else {
                    return Vec::new();
                },
            );
            param_idx += 1;
        }
        // Finally we return suggestions for next argument
        cmd.parameter(&args, param_idx)
            .unwrap()
            .candidates_for(components.last().unwrap())
    }
    fn try_autocomplete(&self) -> Result<String, &'static str> {
        let components = self.command_components();
        let cmd_sugg = CommandNameCompletion(&self.commands).try_autocomplete(components[0])?;
        let cmd = self
            .commands
            .iter()
            .find(|c| c.name() == cmd_sugg.value)
            .unwrap();
        let mut args = Vec::new();
        let mut param_idx = 0;
        while param_idx + 1 < components.len() {
            let param = if let Some(arg) = cmd.parameter(&args, param_idx) {
                arg
            } else {
                break;
            };
            args.push(param.try_autocomplete(components[param_idx + 1])?.value);
            param_idx += 1;
        }
        let mut new_comps = Vec::with_capacity(args.len() + 1);
        new_comps.push(cmd_sugg.value);
        for arg in args {
            new_comps.push(arg);
        }
        Ok(new_comps.join(" "))
    }
    fn execute(&self, kglobals: *mut klib::KGlobals<HW>) -> Result<(), &'static str> {
        let components = self.command_components();
        let cmd_sugg = CommandNameCompletion(&self.commands).try_autocomplete(components[0])?;
        let cmd = self
            .commands
            .iter()
            .find(|c| c.name() == cmd_sugg.value)
            .unwrap();
        cmd.execute(
            &components[1..]
                .iter()
                .map(|s| s.to_string())
                .collect::<Vec<String>>(),
            kglobals,
        );
        Ok(())
    }
    fn prompt(&mut self, kglobals: *mut klib::KGlobals<HW>) {
        print!("diag> {}", self.command_buf);
        'input: loop {
            let c = klib::console::read().unwrap();
            match c {
                '\r' => {
                    println!();
                    if !self.command_buf.trim().is_empty() {
                        match self.try_autocomplete() {
                            Ok(v) => {
                                self.command_buf = v;
                                if let Err(e) = self.execute(kglobals) {
                                    println!("bad command: {}: {}", &self.command_buf, e)
                                }
                            }
                            Err(e) => println!("bad command: {}: {}", &self.command_buf, e),
                        }
                        self.command_buf.clear();
                    }
                    break 'input;
                }
                '\t' => {
                    let candidates = self.candidates();
                    if candidates.len() == 1 {
                        let remaining =
                            &candidates[0].value[self.command_components().last().unwrap().len()..];
                        self.command_buf.push_str(remaining);
                        print!("{}", remaining);
                    } else if !candidates.is_empty() {
                        println!();
                        let max_len = candidates.iter().map(|s| s.value.len()).max().unwrap_or(0);
                        for candidate in &candidates {
                            let name_padded = candidate.value.clone()
                                + (&" ".repeat(max_len - candidate.value.len()));
                            println!(" {} - {}", &name_padded, candidate.help);
                        }
                        break 'input;
                    }
                }
                '\x7f' => {
                    if !self.command_buf.is_empty() {
                        self.command_buf.pop();
                        print!("\x08 \x08");
                    }
                }
                _ => {
                    print!("{}", c);
                    self.command_buf.push(c);
                }
            }
        }
    }
    /// # Safety
    /// `kglobals` must point to the live kernel globals for the shell's lifetime.
    /// Contextual commands may retain and access it across thread switches.
    pub unsafe fn run(&mut self, kglobals: *mut klib::KGlobals<HW>) -> ! {
        loop {
            self.prompt(kglobals)
        }
    }
}

impl<HW: klib::hal::Hardware> Default for Shell<HW> {
    fn default() -> Self {
        Self::new()
    }
}
