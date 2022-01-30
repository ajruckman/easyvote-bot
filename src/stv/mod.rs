use std::io::Write;
use std::process::{Command, Stdio};
use std::str;

pub struct Election {
    votes: Vec<Vec<String>>,
    seats: u8,
}

impl Election {
    pub fn new(votes: Vec<Vec<String>>, seats: u8) -> Self {
        Self {
            votes,
            seats,
        }
    }

    pub fn winners(&self) -> anyhow::Result<Vec<(String, u16)>> {
        let mut child = Command::new("/usr/bin/python3").arg("cmu-frv/run.py")
            .arg("-r")
            .args(&["-s", &self.seats.to_string()])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();

        let stdin = child.stdin.as_mut().unwrap();
        // let stdout = child.stdout.as_mut().unwrap();

        for vote in &self.votes {
            let j = vote.join(",");
            let j = j + "\n";
            stdin.write_all(j.as_bytes()).unwrap();
        }
        stdin.write_all(b"\n").unwrap();

        let output = child.wait_with_output().unwrap();

        let stdout = str::from_utf8(&output.stdout)?.trim();
        println!("{}", stdout);
        let lines = stdout.split("\n");

        let mut result = Vec::new();
        for line in lines {
            let (candidate, votes) = match line.split_once("<|>") {
                None => return Err(anyhow::Error::msg("invalid line: ".to_owned() + line)),
                Some(v) => v,
            };
            let votes = votes.parse::<f32>()?.round() as u16;

            println!("{} => {}", candidate, votes);
            result.push((candidate.to_owned(), votes));
        }

        result.sort_by_key(|(_, v)| -(*v as i32));

        Ok(result)
    }
}
