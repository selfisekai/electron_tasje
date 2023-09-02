use crate::environment::Environment;
use anyhow::{bail, Context, Result};
use once_cell::sync::Lazy;
use regex::{Captures, Regex};
use std::env;

static TEMPLATE_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r#"\$\{([a-zA-Z_. ]+)\}"#).unwrap());

pub(crate) fn fill_variable_template<S: AsRef<str>>(
    template: S,
    environment: Environment,
) -> String {
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

pub fn filesafe_package_name(name: &str) -> Result<String> {
    let new = name.replace('@', "").replace('/', "-");
    if new
        .chars()
        .any(|ch| !ch.is_ascii_alphanumeric() && ch != '-' && ch != '_')
    {
        bail!("invalid package name: {:?}", name);
    }
    Ok(new)
}

#[cfg(test)]
mod tests {
    use super::filesafe_package_name;
    use anyhow::Result;

    #[test]
    fn test_filesafe_name() -> Result<()> {
        assert_eq!(filesafe_package_name("tasje")?, "tasje");
        assert_eq!(
            filesafe_package_name("@bitwarden/desktop")?,
            "bitwarden-desktop"
        );

        Ok(())
    }
}
