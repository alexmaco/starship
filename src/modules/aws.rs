use ini::Ini;
use std::collections::HashMap;
use std::path::PathBuf;
use std::str::FromStr;

use super::{Context, Module, RootModuleConfig};

use crate::configs::aws::AwsConfig;
use crate::formatter::StringFormatter;

type Profile = String;
type Region = String;

async fn get_aws_region_from_config(
    context: &Context<'_>,
    aws_profile: Option<&str>,
) -> Option<Region> {
    let config_location = context
        .get_env("AWS_CONFIG_FILE")
        .and_then(|path| PathBuf::from_str(&path).ok())
        .or_else(|| {
            let mut home = context.get_home()?;
            home.push(".aws/config");
            Some(home)
        })?;

    let ini = async_std::task::spawn(async move { Ini::load_from_file(config_location) })
        .await
        .ok()?;

    let section = if let Some(ref aws_profile) = aws_profile {
        ini.section(Some(format!("profile {}", aws_profile)))
    } else {
        ini.section(Some("default"))
    }?;

    section.get("region").map(String::from)
}

async fn get_aws_profile_and_region(context: &Context<'_>) -> (Option<Profile>, Option<Region>) {
    let profile_env_vars = vec!["AWSU_PROFILE", "AWS_VAULT", "AWS_PROFILE"];
    let profile = profile_env_vars
        .iter()
        .find_map(|env_var| context.get_env(env_var));
    let region = context
        .get_env("AWS_DEFAULT_REGION")
        .or_else(|| context.get_env("AWS_REGION"));
    match (profile, region) {
        (Some(p), Some(r)) => (Some(p), Some(r)),
        (None, Some(r)) => (None, Some(r)),
        (Some(ref p), None) => (
            Some(p.to_owned()),
            get_aws_region_from_config(context, Some(p)).await,
        ),
        (None, None) => (None, get_aws_region_from_config(context, None).await),
    }
}

fn alias_region(region: String, aliases: &HashMap<String, &str>) -> String {
    match aliases.get(&region) {
        None => region,
        Some(alias) => (*alias).to_string(),
    }
}

pub async fn module<'a>(context: &'a Context<'a>) -> Option<Module<'a>> {
    let mut module = context.new_module("aws");
    let config: AwsConfig = AwsConfig::try_load(module.config);

    let (aws_profile, aws_region) = get_aws_profile_and_region(context).await;
    if aws_profile.is_none() && aws_region.is_none() {
        return None;
    }

    let mapped_region = if let Some(aws_region) = aws_region {
        Some(alias_region(aws_region, &config.region_aliases))
    } else {
        None
    };

    let parsed = StringFormatter::new(config.format).and_then(|formatter| {
        formatter
            .map_meta(|variable, _| match variable {
                "symbol" => Some(config.symbol),
                _ => None,
            })
            .map_style(|variable| match variable {
                "style" => Some(Ok(config.style)),
                _ => None,
            })
            .map(|variable| match variable {
                "profile" => aws_profile.as_ref().map(Ok),
                "region" => mapped_region.as_ref().map(Ok),
                _ => None,
            })
            .parse(None)
    });

    module.set_segments(match parsed {
        Ok(segments) => segments,
        Err(error) => {
            log::warn!("Error in module `aws`: \n{}", error);
            return None;
        }
    });

    Some(module)
}

#[cfg(test)]
mod tests {
    use crate::test::ModuleRenderer;
    use ansi_term::Color;
    use std::fs::File;
    use std::io::{self, Write};

    #[test]
    #[ignore]
    fn no_region_set() {
        let actual = ModuleRenderer::new("aws").collect();
        let expected = None;

        assert_eq!(expected, actual);
    }

    #[test]
    fn region_set() {
        let actual = ModuleRenderer::new("aws")
            .env("AWS_REGION", "ap-northeast-2")
            .collect();
        let expected = Some(format!(
            "on {}",
            Color::Yellow.bold().paint("☁️  (ap-northeast-2) ")
        ));

        assert_eq!(expected, actual);
    }

    #[test]
    fn region_set_with_alias() {
        let actual = ModuleRenderer::new("aws")
            .env("AWS_REGION", "ap-southeast-2")
            .config(toml::toml! {
                [aws.region_aliases]
                ap-southeast-2 = "au"
            })
            .collect();
        let expected = Some(format!("on {}", Color::Yellow.bold().paint("☁️  (au) ")));

        assert_eq!(expected, actual);
    }

    #[test]
    fn default_region_set() {
        let actual = ModuleRenderer::new("aws")
            .env("AWS_REGION", "ap-northeast-2")
            .env("AWS_DEFAULT_REGION", "ap-northeast-1")
            .collect();
        let expected = Some(format!(
            "on {}",
            Color::Yellow.bold().paint("☁️  (ap-northeast-1) ")
        ));

        assert_eq!(expected, actual);
    }

    #[test]
    fn profile_set() {
        let actual = ModuleRenderer::new("aws")
            .env("AWS_PROFILE", "astronauts")
            .collect();
        let expected = Some(format!(
            "on {}",
            Color::Yellow.bold().paint("☁️  astronauts ")
        ));

        assert_eq!(expected, actual);
    }

    #[test]
    fn profile_set_from_aws_vault() {
        let actual = ModuleRenderer::new("aws")
            .env("AWS_VAULT", "astronauts-vault")
            .env("AWS_PROFILE", "astronauts-profile")
            .collect();
        let expected = Some(format!(
            "on {}",
            Color::Yellow.bold().paint("☁️  astronauts-vault ")
        ));

        assert_eq!(expected, actual);
    }

    #[test]
    fn profile_set_from_awsu() {
        let actual = ModuleRenderer::new("aws")
            .env("AWSU_PROFILE", "astronauts-awsu")
            .env("AWS_PROFILE", "astronauts-profile")
            .collect();
        let expected = Some(format!(
            "on {}",
            Color::Yellow.bold().paint("☁️  astronauts-awsu ")
        ));

        assert_eq!(expected, actual);
    }

    #[test]
    fn profile_and_region_set() {
        let actual = ModuleRenderer::new("aws")
            .env("AWS_PROFILE", "astronauts")
            .env("AWS_REGION", "ap-northeast-2")
            .collect();
        let expected = Some(format!(
            "on {}",
            Color::Yellow
                .bold()
                .paint("☁️  astronauts (ap-northeast-2) ")
        ));

        assert_eq!(expected, actual);
    }

    #[test]
    fn default_profile_set() -> io::Result<()> {
        let dir = tempfile::tempdir()?;
        let config_path = dir.path().join("config");
        let mut file = File::create(&config_path)?;

        file.write_all(
            "[default]
region = us-east-1

[profile astronauts]
region = us-east-2
"
            .as_bytes(),
        )?;

        let actual = ModuleRenderer::new("aws")
            .env("AWS_CONFIG_FILE", config_path.to_string_lossy().as_ref())
            .collect();
        let expected = Some(format!(
            "on {}",
            Color::Yellow.bold().paint("☁️  (us-east-1) ")
        ));

        assert_eq!(expected, actual);
        dir.close()
    }

    #[test]
    fn profile_and_config_set() -> io::Result<()> {
        let dir = tempfile::tempdir()?;
        let config_path = dir.path().join("config");
        let mut file = File::create(&config_path)?;

        file.write_all(
            "[default]
region = us-east-1

[profile astronauts]
region = us-east-2
"
            .as_bytes(),
        )?;

        let actual = ModuleRenderer::new("aws")
            .env("AWS_CONFIG_FILE", config_path.to_string_lossy().as_ref())
            .env("AWS_PROFILE", "astronauts")
            .config(toml::toml! {
                [aws]
            })
            .collect();
        let expected = Some(format!(
            "on {}",
            Color::Yellow.bold().paint("☁️  astronauts (us-east-2) ")
        ));

        assert_eq!(expected, actual);
        dir.close()
    }

    #[test]
    fn profile_and_region_set_with_display_all() {
        let actual = ModuleRenderer::new("aws")
            .env("AWS_PROFILE", "astronauts")
            .env("AWS_REGION", "ap-northeast-1")
            .collect();
        let expected = Some(format!(
            "on {}",
            Color::Yellow
                .bold()
                .paint("☁️  astronauts (ap-northeast-1) ")
        ));

        assert_eq!(expected, actual);
    }

    #[test]
    fn profile_set_with_display_all() {
        let actual = ModuleRenderer::new("aws")
            .env("AWS_PROFILE", "astronauts")
            .collect();
        let expected = Some(format!(
            "on {}",
            Color::Yellow.bold().paint("☁️  astronauts ")
        ));

        assert_eq!(expected, actual);
    }

    #[test]
    fn region_set_with_display_all() {
        let actual = ModuleRenderer::new("aws")
            .env("AWS_REGION", "ap-northeast-1")
            .collect();
        let expected = Some(format!(
            "on {}",
            Color::Yellow.bold().paint("☁️  (ap-northeast-1) ")
        ));

        assert_eq!(expected, actual);
    }

    #[test]
    fn profile_and_region_set_with_display_region() {
        let actual = ModuleRenderer::new("aws")
            .env("AWS_PROFILE", "astronauts")
            .env("AWS_DEFAULT_REGION", "ap-northeast-1")
            .config(toml::toml! {
                [aws]
                format = "on [$symbol$region]($style) "
            })
            .collect();
        let expected = Some(format!(
            "on {} ",
            Color::Yellow.bold().paint("☁️  ap-northeast-1")
        ));

        assert_eq!(expected, actual);
    }

    #[test]
    fn profile_and_region_set_with_display_profile() {
        let actual = ModuleRenderer::new("aws")
            .env("AWS_PROFILE", "astronauts")
            .env("AWS_REGION", "ap-northeast-1")
            .config(toml::toml! {
                [aws]
                format = "on [$symbol$profile]($style) "
            })
            .collect();
        let expected = Some(format!(
            "on {} ",
            Color::Yellow.bold().paint("☁️  astronauts")
        ));

        assert_eq!(expected, actual);
    }

    #[test]
    fn region_set_with_display_profile() {
        let actual = ModuleRenderer::new("aws")
            .env("AWS_REGION", "ap-northeast-1")
            .config(toml::toml! {
                [aws]
                format = "on [$symbol$profile]($style) "
            })
            .collect();
        let expected = Some(format!("on {} ", Color::Yellow.bold().paint("☁️  ")));

        assert_eq!(expected, actual);
    }

    #[test]
    #[ignore]
    fn region_not_set_with_display_region() {
        let actual = ModuleRenderer::new("aws")
            .config(toml::toml! {
                [aws]
                format = "on [$symbol$region]($style) "
            })
            .collect();
        let expected = None;

        assert_eq!(expected, actual);
    }
}
