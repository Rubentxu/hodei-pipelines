//! Specification Pattern for composable business rules validation

use crate::error::DomainError;
use std::fmt;
use std::marker::PhantomData;

/// Result of specification validation with detailed error information
#[derive(Debug, Clone)]
pub struct SpecificationResult {
    pub errors: Vec<String>,
}

impl SpecificationResult {
    pub fn new() -> Self {
        Self { errors: Vec::new() }
    }

    pub fn add_error(&mut self, error: String) {
        self.errors.push(error);
    }

    pub fn is_valid(&self) -> bool {
        self.errors.is_empty()
    }

    pub fn to_result(&self) -> Result<(), DomainError> {
        if self.errors.is_empty() {
            Ok(())
        } else {
            Err(DomainError::Validation(self.errors.join(", ")))
        }
    }

    pub fn satisfied() -> Self {
        Self { errors: Vec::new() }
    }

    pub fn failed(errors: Vec<String>) -> Self {
        Self { errors }
    }
}

impl Default for SpecificationResult {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper struct to build and evaluate composite specifications
pub struct SpecificationResultBuilder<'a, T> {
    candidate: &'a T,
    errors: Vec<String>,
}

impl<'a, T> SpecificationResultBuilder<'a, T> {
    pub fn new(candidate: &'a T) -> Self {
        Self {
            candidate,
            errors: Vec::new(),
        }
    }

    pub fn add_error<S: Into<String>>(&mut self, error: S) {
        self.errors.push(error.into());
    }

    pub fn is_satisfied(&self) -> bool {
        self.errors.is_empty()
    }

    pub fn build(self) -> SpecificationResult {
        SpecificationResult {
            errors: self.errors,
        }
    }
}

/// Specification trait for composable validation rules
pub trait Specification<T> {
    /// Check if the candidate satisfies this specification
    fn is_satisfied_by(&self, candidate: &T) -> bool;

    /// Combine this specification with another using AND logic
    fn and<U>(self, other: U) -> AndSpec<Self, U, T>
    where
        Self: Sized,
        U: Specification<T>,
    {
        AndSpec::new(self, other)
    }

    /// Combine this specification with another using OR logic
    fn or<U>(self, other: U) -> OrSpec<Self, U, T>
    where
        Self: Sized,
        U: Specification<T>,
    {
        OrSpec::new(self, other)
    }

    /// Negate this specification using NOT logic
    fn not(self) -> NotSpec<Self, T>
    where
        Self: Sized,
    {
        NotSpec::new(self)
    }
}

/// Composite specification for AND logic
#[derive(Debug)]
pub struct AndSpec<A, B, T> {
    left: A,
    right: B,
    phantom: PhantomData<T>,
}

impl<A, B, T> AndSpec<A, B, T> {
    pub fn new(left: A, right: B) -> Self {
        Self {
            left,
            right,
            phantom: PhantomData,
        }
    }
}

impl<A, B, T> Specification<T> for AndSpec<A, B, T>
where
    A: Specification<T>,
    B: Specification<T>,
{
    fn is_satisfied_by(&self, candidate: &T) -> bool {
        self.left.is_satisfied_by(candidate) && self.right.is_satisfied_by(candidate)
    }
}

impl<A, B, T> fmt::Display for AndSpec<A, B, T>
where
    A: fmt::Display,
    B: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({} AND {})", self.left, self.right)
    }
}

/// Composite specification for OR logic
#[derive(Debug)]
pub struct OrSpec<A, B, T> {
    left: A,
    right: B,
    phantom: PhantomData<T>,
}

impl<A, B, T> OrSpec<A, B, T> {
    pub fn new(left: A, right: B) -> Self {
        Self {
            left,
            right,
            phantom: PhantomData,
        }
    }
}

impl<A, B, T> Specification<T> for OrSpec<A, B, T>
where
    A: Specification<T>,
    B: Specification<T>,
{
    fn is_satisfied_by(&self, candidate: &T) -> bool {
        self.left.is_satisfied_by(candidate) || self.right.is_satisfied_by(candidate)
    }
}

impl<A, B, T> fmt::Display for OrSpec<A, B, T>
where
    A: fmt::Display,
    B: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({} OR {})", self.left, self.right)
    }
}

/// Composite specification for NOT logic
#[derive(Debug)]
pub struct NotSpec<A, T> {
    inner: A,
    phantom: PhantomData<T>,
}

impl<A, T> NotSpec<A, T> {
    pub fn new(inner: A) -> Self {
        Self {
            inner,
            phantom: PhantomData,
        }
    }
}

impl<A, T> Specification<T> for NotSpec<A, T>
where
    A: Specification<T>,
{
    fn is_satisfied_by(&self, candidate: &T) -> bool {
        !self.inner.is_satisfied_by(candidate)
    }
}

impl<A, T> fmt::Display for NotSpec<A, T>
where
    A: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "NOT ({})", self.inner)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct EvenNumberSpec;

    impl Specification<i32> for EvenNumberSpec {
        fn is_satisfied_by(&self, candidate: &i32) -> bool {
            *candidate % 2 == 0
        }
    }

    struct PositiveNumberSpec;

    impl Specification<i32> for PositiveNumberSpec {
        fn is_satisfied_by(&self, candidate: &i32) -> bool {
            *candidate > 0
        }
    }

    #[test]
    fn test_and_spec() {
        let spec = EvenNumberSpec.and(PositiveNumberSpec);
        assert!(spec.is_satisfied_by(&4));
        assert!(!spec.is_satisfied_by(&-2));
        assert!(!spec.is_satisfied_by(&3));
    }

    #[test]
    fn test_or_spec() {
        let spec = EvenNumberSpec.or(PositiveNumberSpec);
        assert!(spec.is_satisfied_by(&3));
        assert!(spec.is_satisfied_by(&-2));
        assert!(!spec.is_satisfied_by(&-3));
    }

    #[test]
    fn test_not_spec() {
        let spec = PositiveNumberSpec.not();
        assert!(!spec.is_satisfied_by(&5));
        assert!(spec.is_satisfied_by(&-5));
        assert!(spec.is_satisfied_by(&0));
    }
}
