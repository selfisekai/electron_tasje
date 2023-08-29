use std::env;

use anyhow::Context;
use once_cell::sync::Lazy;
use regex::{Captures, Regex};

use crate::environment::Environment;

static TEMPLATE_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r#"\$\{([a-zA-Z_. ]+)\}"#).unwrap());

pub fn fill_variable_template<S: AsRef<str>>(template: S, environment: Environment) -> String {
    TEMPLATE_REGEX
        .replace_all(template.as_ref(), |captures: &Captures| -> String {
            let variable = captures.get(1).unwrap().as_str().trim();
            match variable {
                "arch" => environment.architecture.to_node().to_string(),
                "platform" => environment.platform.to_node().to_string(),
                v => {
                    if let Some(envar) = v.strip_prefix("env.") {
                        env::var(envar)
                            .with_context(|| format!("variable name: {:?}", envar))
                            .expect("getting env variable contents")
                    } else {
                        unimplemented!("unknown template variable: '{variable}'")
                    }
                }
            }
        })
        .to_string()
}
