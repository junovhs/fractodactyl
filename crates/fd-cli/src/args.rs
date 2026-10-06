//! `--key value` flags and positional arguments, without a parser dependency.
use std::collections::HashMap;
use std::str::FromStr;

pub(crate) struct Args {
    flags: HashMap<String, String>,
    pub(crate) positional: Vec<String>,
}

impl Args {
    pub(crate) fn parse(argv: &[String], known: &[&str]) -> Result<Args, String> {
        let (mut flags, mut positional) = (HashMap::new(), Vec::new());
        let mut it = argv.iter();
        while let Some(a) = it.next() {
            if let Some(k) = a.strip_prefix("--").or_else(|| a.strip_prefix('-').filter(|k| k.len() == 1)) {
                if !known.contains(&k) {
                    return Err(format!("unknown flag {a}"));
                }
                let v = it.next().ok_or_else(|| format!("{a} needs a value"))?;
                flags.insert(k.to_string(), v.clone());
            } else {
                positional.push(a.clone());
            }
        }
        Ok(Args { flags, positional })
    }

    pub(crate) fn str(&self, k: &str) -> Option<&str> {
        self.flags.get(k).map(String::as_str)
    }

    pub(crate) fn need(&self, k: &str) -> Result<&str, String> {
        self.str(k).ok_or_else(|| format!("--{k} is required"))
    }

    pub(crate) fn num<T: FromStr>(&self, k: &str, default: T) -> Result<T, String> {
        match self.str(k) {
            None => Ok(default),
            Some(v) => v.parse().map_err(|_| format!("--{k}: bad number {v:?}")),
        }
    }
}
