//! Command line tool with utilities to make working with the courses in this repository easier.

use std::{collections::BTreeMap, fs, vec};

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use serde::ser::Serialize;
use trane::{
    course_library::CourseLibrary,
    data::{
        course_generator::transcription::{
            TranscriptionAsset, TranscriptionConfig, TranscriptionLink,
        },
        CourseGenerator, CourseManifestBuilder,
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

    // Generate the course manifest with the required fields filled in.
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
    // Open the trane-transcription library in trane. This requires that the command is run in the
    // root of the repository.
    let _ = Trane::new_local(&std::env::current_dir()?, &std::env::current_dir()?)?;
    Ok(())
}

/// Verifies that a YouTube link refers to a valid video.
fn verify_youtube_link(link: &str) -> Result<()> {
    // Use the oembed format to retrieve a small amount of data.
    let url = format!("https://www.youtube.com/oembed?url={link}&format=json");
    let res = ureq::get(&url)
        .set("Example-Header", "header value")
        .call()?;
    if res.status() != 200 {
        bail!("Invalid YouTube link: {}", link);
    }
    Ok(())
}

/// Verifies that all links in the transcription courses are valid.
fn verify_links() -> Result<()> {
    // Open the trane-transcription library in trane. This requires that the command is run in the
    // root of the repository.
    let trane = Trane::new_local(&std::env::current_dir()?, &std::env::current_dir()?)?;

    // Go through each course and verify that all external links are valid.
    let courses = trane.get_course_ids();
    let mut invalid_links = 0;
    for course_id in courses {
        let manifest = trane.get_course_manifest(course_id).unwrap();
        if manifest.generator_config.is_none() {
            continue;
        }

        if let CourseGenerator::Transcription(config) = manifest.generator_config.unwrap() {
            for passages in config.inlined_passages {
                match passages.asset {
                    TranscriptionAsset::Track {
                        short_id,
                        external_link,
                        ..
                    } => {
                        if let Some(link) = external_link {
                            match link {
                                TranscriptionLink::YouTube(yt_link) => {
                                    let valid = verify_youtube_link(&yt_link);
                                    if valid.is_err() {
                                        invalid_links += 1;
                                        println!(
                                            "Course {}, asset {} has an invalid YouTube link.",
                                            course_id, short_id
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    if invalid_links == 0 {
        println!("All courses have valid links.");
    }
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

    #[clap(about = "Verify that all links in the transcription courses are valid")]
    VerifyLinks,
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

            Subcommands::VerifyLinks => verify_links()?,
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
