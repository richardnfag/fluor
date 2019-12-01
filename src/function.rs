use std::fs::{remove_file, File};
use std::io::prelude::Write;
use std::path::Path;
use std::process::Command;

use base64::decode;
use futures::executor::block_on;
use futures_util::TryStreamExt;
use serde_derive::Deserialize;

use hyper::{Body, Response};

use hyper::http::request::Parts;

use crate::router::Router;
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

    pub fn build(self) -> Result<Function, &'static str> {
        match File::create(&Path::new(&"data/source.tar.gz"))
            .map(|mut f| decode(self.source.as_str()).map(|s| f.write_all(s.as_slice())))
        {
            Ok(Err(_)) => {
                return Err("Error: Could not decode file");
            }
            Err(_) => {
                return Err("Error: Could not create file");
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
            return Err("Failed to execute: docker build");
        }

        remove_file("data/source.tar.gz").ok();

        Ok(self)
    }

    pub fn run(&self, _headers: Parts, body: Body) -> Response<Body> {
        let b = block_on(async { body.try_concat().await.unwrap() });

        let req = String::from_utf8_lossy(&b.into_bytes()).to_string();

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
            println!("{}", String::from_utf8_lossy(&output.stderr).to_string());
            return Response::builder()
                .status(500)
                .header("Access-Control-Allow-Origin", "*")
                .body("Internal Server Error".into())
                .unwrap();
        }

        Response::builder()
            .status(200)
            .header("Access-Control-Allow-Origin", "*")
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
