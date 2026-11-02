/// A bounded navigation stack with an immutable root position.
#[derive(Debug)]
pub struct ScreenStack<S> {
    screens: Vec<S>,
    max_depth: usize,
}

impl<S> ScreenStack<S> {
    /// `max_depth` includes the root; zero is invalid.
    /// # Errors
    /// Returns an error when the depth is zero.
    pub fn new(root: S, max_depth: usize) -> std::io::Result<Self> {
        if max_depth == 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "screen depth must be nonzero",
            ));
        }
        Ok(Self {
            screens: vec![root],
            max_depth,
        })
    }
    #[must_use]
    pub fn current(&self) -> Option<&S> {
        self.screens.last()
    }
    /// Returns the unconsumed screen if the stack is full.
    /// # Errors
    /// Returns the supplied screen when the configured depth is reached.
    pub fn push(&mut self, screen: S) -> Result<(), S> {
        if self.screens.len() >= self.max_depth {
            Err(screen)
        } else {
            self.screens.push(screen);
            Ok(())
        }
    }
    /// Pop the current screen but never the root.
    pub fn pop(&mut self) -> Option<S> {
        if self.screens.len() > 1 {
            self.screens.pop()
        } else {
            None
        }
    }
}
