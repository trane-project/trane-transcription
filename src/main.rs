fn main() {}

#[cfg(test)]
mod test {
    use anyhow::Result;
    use trane::{course_library::CourseLibrary, Trane};

    #[test]
    fn verify_courses() -> Result<()> {
        let trane = Trane::new_local(&std::env::current_dir()?, &std::env::current_dir()?)?;
        assert!(trane.get_all_exercise_ids(None).len() > 0);
        Ok(())
    }
}
