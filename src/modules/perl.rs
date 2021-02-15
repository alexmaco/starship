use super::{Context, Module, RootModuleConfig};

use crate::configs::perl::PerlConfig;
use crate::formatter::StringFormatter;

/// Creates a module with the current perl version
pub async fn module<'a>(context: &'a Context<'a>) -> Option<Module<'a>> {
    let mut module = context.new_module("perl");
    let config: PerlConfig = PerlConfig::try_load(module.config);
    let is_perl_project = context
        .try_begin_scan()?
        .set_extensions(&config.detect_extensions)
        .set_files(&config.detect_files)
        .set_folders(&config.detect_folders)
        .is_match();

    if !is_perl_project {
        return None;
    }

    let parsed = match StringFormatter::new(config.format) {
        Ok(formatter) => formatter
            .map_meta(|var, _| match var {
                "symbol" => Some(config.symbol),
                _ => None,
            })
            .map_style(|variable| match variable {
                "style" => Some(Ok(config.style)),
                _ => None,
            })
            .async_map(|variable| async move {
                match variable.as_ref() {
                    "version" => {
                        let perl_version = context
                            .async_exec_cmd("perl", &["-e", "printf q#%vd#,$^V;"])
                            .await?
                            .stdout;
                        Some(Ok(format!("v{}", perl_version)))
                    }
                    _ => None,
                }
            })
            .await
            .parse(None),
        Err(e) => Err(e),
    };

    module.set_segments(match parsed {
        Ok(segments) => segments,
        Err(error) => {
            log::warn!("Error in module `perl`:\n{}", error);
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
    use std::io;

    #[test]
    fn folder_without_perl_files() -> io::Result<()> {
        let dir = tempfile::tempdir()?;

        let actual = ModuleRenderer::new("perl").path(dir.path()).collect();

        let expected = None;
        assert_eq!(expected, actual);
        dir.close()
    }

    #[test]
    fn folder_with_makefile_file() -> io::Result<()> {
        let dir = tempfile::tempdir()?;
        File::create(dir.path().join("Makefile.PL"))?.sync_all()?;

        let actual = ModuleRenderer::new("perl").path(dir.path()).collect();

        let expected = Some(format!(
            "via {}",
            Color::Fixed(149).bold().paint("🐪 v5.26.1 ")
        ));
        assert_eq!(expected, actual);
        dir.close()
    }

    #[test]
    fn folder_with_buildfile_file() -> io::Result<()> {
        let dir = tempfile::tempdir()?;
        File::create(dir.path().join("Build.PL"))?.sync_all()?;

        let actual = ModuleRenderer::new("perl").path(dir.path()).collect();

        let expected = Some(format!(
            "via {}",
            Color::Fixed(149).bold().paint("🐪 v5.26.1 ")
        ));
        assert_eq!(expected, actual);
        dir.close()
    }

    #[test]
    fn folder_with_cpanfile_file() -> io::Result<()> {
        let dir = tempfile::tempdir()?;
        File::create(dir.path().join("cpanfile"))?.sync_all()?;

        let actual = ModuleRenderer::new("perl").path(dir.path()).collect();

        let expected = Some(format!(
            "via {}",
            Color::Fixed(149).bold().paint("🐪 v5.26.1 ")
        ));
        assert_eq!(expected, actual);
        dir.close()
    }

    #[test]
    fn folder_with_cpanfile_snapshot_file() -> io::Result<()> {
        let dir = tempfile::tempdir()?;
        File::create(dir.path().join("cpanfile.snapshot"))?.sync_all()?;

        let actual = ModuleRenderer::new("perl").path(dir.path()).collect();

        let expected = Some(format!(
            "via {}",
            Color::Fixed(149).bold().paint("🐪 v5.26.1 ")
        ));
        assert_eq!(expected, actual);
        dir.close()
    }

    #[test]
    fn folder_with_meta_json_file() -> io::Result<()> {
        let dir = tempfile::tempdir()?;
        File::create(dir.path().join("META.json"))?.sync_all()?;

        let actual = ModuleRenderer::new("perl").path(dir.path()).collect();

        let expected = Some(format!(
            "via {}",
            Color::Fixed(149).bold().paint("🐪 v5.26.1 ")
        ));
        assert_eq!(expected, actual);
        dir.close()
    }

    #[test]
    fn folder_with_meta_yml_file() -> io::Result<()> {
        let dir = tempfile::tempdir()?;
        File::create(dir.path().join("META.yml"))?.sync_all()?;

        let actual = ModuleRenderer::new("perl").path(dir.path()).collect();

        let expected = Some(format!(
            "via {}",
            Color::Fixed(149).bold().paint("🐪 v5.26.1 ")
        ));
        assert_eq!(expected, actual);
        dir.close()
    }

    #[test]
    fn folder_with_perl_version() -> io::Result<()> {
        let dir = tempfile::tempdir()?;
        File::create(dir.path().join(".perl-version"))?.sync_all()?;

        let actual = ModuleRenderer::new("perl").path(dir.path()).collect();

        let expected = Some(format!(
            "via {}",
            Color::Fixed(149).bold().paint("🐪 v5.26.1 ")
        ));
        assert_eq!(expected, actual);
        dir.close()
    }

    #[test]
    fn folder_with_perl_file() -> io::Result<()> {
        let dir = tempfile::tempdir()?;
        File::create(dir.path().join("any.pl"))?.sync_all()?;

        let actual = ModuleRenderer::new("perl").path(dir.path()).collect();

        let expected = Some(format!(
            "via {}",
            Color::Fixed(149).bold().paint("🐪 v5.26.1 ")
        ));
        assert_eq!(expected, actual);
        dir.close()
    }

    #[test]
    fn folder_with_perl_module_file() -> io::Result<()> {
        let dir = tempfile::tempdir()?;
        File::create(dir.path().join("any.pm"))?.sync_all()?;

        let actual = ModuleRenderer::new("perl").path(dir.path()).collect();

        let expected = Some(format!(
            "via {}",
            Color::Fixed(149).bold().paint("🐪 v5.26.1 ")
        ));
        assert_eq!(expected, actual);
        dir.close()
    }

    #[test]
    fn folder_with_perldoc_file() -> io::Result<()> {
        let dir = tempfile::tempdir()?;
        File::create(dir.path().join("any.pod"))?.sync_all()?;

        let actual = ModuleRenderer::new("perl").path(dir.path()).collect();

        let expected = Some(format!(
            "via {}",
            Color::Fixed(149).bold().paint("🐪 v5.26.1 ")
        ));
        assert_eq!(expected, actual);
        dir.close()
    }
}
