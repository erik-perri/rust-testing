use std::any::Any;
use std::thread::JoinHandle;

pub struct ThreadJoiner {
    threads: Vec<JoinHandle<()>>,
}

impl ThreadJoiner {
    pub fn new(threads: Vec<JoinHandle<()>>) -> Self {
        Self { threads }
    }

    pub fn join(self) -> Result<(), Box<dyn Any + Send>> {
        for thread in self.threads {
            thread.join()?;
        }

        Ok(())
    }
}
