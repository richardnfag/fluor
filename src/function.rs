use crate::router::Router;
use std::fs::{remove_file, File};
use std::io::prelude::Write;
use std::path::Path;
use std::process::Command;

use base64::decode;
use futures::{Future, Stream};
use serde_derive::Deserialize;

use hyper::{Body, Response};

use hyper::http::request::Parts;

use crate::trigger::Trigger;

#[derive(Clone, Debug, Deserialize)]
pub struct Function {
    name: String,
    language: String,
    source: String,
    method: String,
    path: String,
    cpu: String,
    memory: String,
    uptime: String,
}

impl Function {
    pub fn from_json(b: &[u8]) -> Option<Function> {
        serde_json::from_slice(b).ok()
    }

    pub fn trigger(&self) -> Trigger {
        Trigger::new(self.method.as_str(), self.path.as_str())
    }

    pub fn build(self) -> Option<Function> {
        match File::create(&Path::new(&"data/source.tar.gz"))
            .map(|mut f| decode(self.source.as_str()).map(|s| f.write_all(s.as_slice())))
        {
            Ok(Err(_)) => {
                eprintln!("Error: Could not decode file");
                return None;
            }
            Err(_) => {
                eprintln!("Error: Could not create file");
                return None;
            }
            _ => {}
        };

        if !Command::new("docker")
            .arg("build")
            .arg("-f")
            .arg(format!("templates/{}.dockerfile", self.language))
            .arg("-t")
            .arg(format!("function-{}", self.name))
            .arg("data/")
            .status()
            .expect("Failed to execute: docker build")
            .success()
        {
            return None;
        }

        remove_file("data/source.tar.gz").ok();

        Some(self)
    }

    pub fn run(&self, _headers: Parts, body: Body) -> Response<Body> {
        let req = body.concat2().wait().ok().map_or(String::new(), |b| {
            String::from_utf8_lossy(b.as_ref()).to_string()
        });

        let output = Command::new("timeout")
            .arg("--signal=SIGKILL")
            .arg(&self.uptime)
            .arg("docker")
            .arg("run")
            .arg("--rm")
            .arg(format!("--cpus={}", self.cpu))
            .arg(format!("--memory={}", self.memory))
            .arg(format!("function-{}", self.name))
            .env("FNREQUEST", req)
            .output()
            .expect("Failed to execute: docker run");

        if !output.status.success() {
            return Response::builder()
                .status(500)
                .body("Internal Server Error".into())
                .unwrap();
        }

        Response::builder()
            .body(String::from_utf8_lossy(&output.stdout).to_string().into())
            .unwrap()
    }

    pub fn delete(&self, router: Router) -> Response<Body> {
        let output = Command::new("docker")
            .arg("rmi")
            .arg(format!("function-{}", self.name))
            .output()
            .expect("Failed to execute: docker rmi!");

        if !output.status.success() {
            return Response::builder()
                .status(500)
                .body("Internal Server Error".into())
                .unwrap();
        }
        router.remove(&self.trigger());
        Response::new("Function Deleted".into())
    }

    pub fn path(&self) -> &str {
        self.path.as_str()
    }

    pub fn name(&self) -> &str {
        self.name.as_str()
    }
}
