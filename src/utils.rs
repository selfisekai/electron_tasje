use crate::environment::Environment;
use anyhow::{bail, Context, Result};
use once_cell::sync::Lazy;
use regex::{Captures, Regex};
use std::env;

static TEMPLATE_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"\$\{([a-zA-Z_. ]+)\}").unwrap());

pub(crate) fn try_flatten<S, T>(iter: S) -> Result<Vec<T>>
where
    S: Iterator<Item = Result<T>>,
{
    let mut unwrapped = Vec::new();
    for item in iter {
        match item {
            Ok(i) => unwrapped.push(i),
            Err(e) => return Err(e),
        }
    }
    Ok(unwrapped)
}

/// from regex crate docs
fn replace_all<E>(
    re: &Regex,
    haystack: &str,
    replacement: impl Fn(&Captures) -> Result<String, E>,
) -> Result<String, E> {
    let mut new = String::with_capacity(haystack.len());
    let mut last_match = 0;
    for caps in re.captures_iter(haystack) {
        let m = caps.get(0).unwrap();
        new.push_str(&haystack[last_match..m.start()]);
        new.push_str(&replacement(&caps)?);
        last_match = m.end();
    }
    new.push_str(&haystack[last_match..]);
    Ok(new)
}

pub(crate) fn fill_variable_template<S: AsRef<str>>(
    template: S,
    environment: Environment,
) -> Result<String> {
    replace_all(
        &TEMPLATE_REGEX,
        template.as_ref(),
        |captures: &Captures| -> Result<String> {
            let variable = captures.get(1).unwrap().as_str().trim();
            match variable {
                "arch" => Ok(environment.architecture.to_node().to_string()),
                "platform" => Ok(environment.platform.to_node().to_string()),
                v => {
                    if let Some(envar) = v.strip_prefix("env.") {
                        env::var(envar)
                            .with_context(|| format!("failed to get the env variable: {:?}", envar))
                    } else {
                        bail!("unknown template variable: '{variable}'")
                    }
                }
            }
        },
    )
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
    use super::{filesafe_package_name, fill_variable_template};
    use crate::environment::Environment;
    use anyhow::Result;

    #[test]
    fn test_variable_templates() -> Result<()> {
        let env = Environment {
            architecture: crate::environment::Architecture::Aarch64,
            platform: crate::environment::Platform::Linux,
        };
        assert_eq!(fill_variable_template("tasje", env)?, "tasje");
        assert_eq!(
            fill_variable_template("tasje-${arch}-${platform}", env)?,
            "tasje-arm64-linux"
        );
        assert_eq!(
            fill_variable_template("_${env.CARGO_PKG_NAME}_", env)?,
            "_electron_tasje_"
        );

        Ok(())
    }

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
