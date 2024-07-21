//! Command line tool with utilities to make working with the courses in this repository easier.

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use trane::{
    data::{
        course_generator::transcription::TranscriptionConfig, CourseGenerator, CourseManifest,
        CourseManifestBuilder,
    },
    Trane,
};
use ustr::Ustr;

/// Creates a new course with the basic details filled in.
fn create_course(id: &str) -> Result<()> {
    // Check the required courses are available.
    let root = std::env::current_dir()?.join("courses");
    if !root.exists() {
        bail!("courses directory does not exist at {}", root.display());
    }
    let directory = if id.starts_with("trane::transcription::") {
        let path = id
            .trim_start_matches("trane::transcription::")
            .split("::")
            .collect::<Vec<_>>()
            .join("/");
        root.join(path)
    } else {
        let path = id.split("::").collect::<Vec<_>>().join("/");
        root.join(path)
    };
    if directory.exists() {
        bail!("course already exists at {}", directory.display());
    }

    // Generate the course manifest.
    let course_id = if id.starts_with("trane::transcription::") {
        Ustr::from(id)
    } else {
        Ustr::from(&format!("trane::transcription::{id}"))
    };
    let course_manifest = CourseManifestBuilder::default()
        .id(course_id)
        .generator_config(Some(CourseGenerator::Transcription(TranscriptionConfig {
            transcription_dependencies: vec![],
            passage_directory: "".to_string(),
            inlined_passages: vec![],
            skip_singing_lessons: false,
            skip_advanced_lessons: false,
        })))
        .build()
        .with_context(|| "failed to build course manifest")?;

    let pretty_json = serde_json::to_string_pretty(&course_manifest)
        .with_context(|| "invalid course manifest")?;
    // fs::write(&path, pretty_json)
    //     .with_context(|| format!("failed to write metadata to {}", path.display()))?;
    Ok(())
}

/// Verifies that all transcription courses are valid.
fn verify_courses() -> Result<()> {
    let _ = Trane::new_local(&std::env::current_dir()?, &std::env::current_dir()?)?;
    Ok(())
}

#[derive(Debug, Parser)]
#[clap(name = "transcription-cli")]
#[clap(author, version, about, long_about = None)]
pub(crate) struct TranscriptionCLI {
    #[clap(subcommand)]
    pub commands: Subcommands,
}

/// Contains the available subcommands.
#[derive(Clone, Debug, Subcommand)]
pub(crate) enum Subcommands {
    #[clap(about = "Create a new transcription course")]
    New {
        #[clap(help = "The id of the course to create without the trane::transcription:: prefix")]
        id: String,
    },

    #[clap(about = "Verify that all transcription courses are valid")]
    VerifyCourses,
}

impl Subcommands {
    /// Executes the subcommand.
    pub fn execute(&self) -> Result<()> {
        match self {
            Subcommands::New { id } => create_course(id)?,

            Subcommands::VerifyCourses => match verify_courses() {
                Ok(_) => println!("All courses are valid."),
                Err(e) => eprintln!("Error validating courses: {e}"),
            },
        }
        Ok(())
    }
}

fn main() -> Result<()> {
    let args = TranscriptionCLI::parse();
    args.commands.execute()?;
    Ok(())
}

#[cfg(test)]
mod test {
    use anyhow::Result;
    use trane::{course_library::CourseLibrary, Trane};

    #[test]
    fn test_verify_courses() -> Result<()> {
        let trane = Trane::new_local(&std::env::current_dir()?, &std::env::current_dir()?)?;
        assert!(trane.get_all_exercise_ids(None).len() > 0);
        Ok(())
    }
}
