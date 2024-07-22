//! Command line tool with utilities to make working with the courses in this repository easier.

use std::{collections::BTreeMap, fs, vec};

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use serde::ser::Serialize;
use trane::{
    data::{
        course_generator::transcription::TranscriptionConfig, CourseGenerator,
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
        .authors(Some(vec!["The Trane Project".to_string()]))
        .metadata(Some(BTreeMap::from([(
            "course_series".to_string(),
            vec!["trane_transcription".to_string()],
        )])))
        .generator_config(Some(CourseGenerator::Transcription(TranscriptionConfig {
            transcription_dependencies: vec![],
            passage_directory: "".to_string(),
            inlined_passages: vec![],
            skip_singing_lessons: false,
            skip_advanced_lessons: false,
        })))
        .build()
        .with_context(|| "failed to build course manifest")?;

    // Create the directory and write the course manifest.
    fs::create_dir_all(&directory).with_context(|| {
        format!(
            "failed to create course directory at {}",
            directory.display()
        )
    })?;
    let formatter = serde_json::ser::PrettyFormatter::with_indent(b"    ");
    let mut buf = Vec::new();
    let mut ser = serde_json::Serializer::with_formatter(&mut buf, formatter);
    course_manifest
        .serialize(&mut ser)
        .with_context(|| "failed to serialize repository metadata")?;
    let manifest_path = directory.join("course_manifest.json");
    fs::write(&manifest_path, buf).with_context(|| {
        format!(
            "failed to write course metadata to {}",
            manifest_path.display()
        )
    })?;
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
        #[clap(
            help = "The id of the course to create with or without the trane::transcription:: \
            prefix"
        )]
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
