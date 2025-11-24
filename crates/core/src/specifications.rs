//! Specification Pattern for composable business rules validation
//!
//! This module implements the Specification pattern, allowing for modular,
//! composable, and testable validation rules that can be combined using
//! logical operators (AND, OR, NOT).

use crate::DomainError;
use std::fmt;
use std::marker::PhantomData;

/// Specification trait - defines the contract for business rule specifications
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

/// Helper struct to build and evaluate composite specifications
pub struct SpecificationResult<'a, T> {
    candidate: &'a T,
    errors: Vec<String>,
}

impl<'a, T> SpecificationResult<'a, T> {
    pub fn new(candidate: &'a T) -> Self {
        Self {
            candidate,
            errors: Vec::new(),
        }
    }

    pub fn add_error<S: Into<String>>(&mut self, error: S) {
        self.errors.push(error.into());
    }

    pub fn is_valid(&self) -> bool {
        self.errors.is_empty()
    }

    pub fn errors(&self) -> &[String] {
        &self.errors
    }

    pub fn into_result(self) -> Result<&'a T, DomainError> {
        if self.errors.is_empty() {
            Ok(self.candidate)
        } else {
            Err(DomainError::Validation(self.errors.join("; ")))
        }
    }
}

/// Validate a candidate against multiple specifications
pub fn validate_specifications<'a, T, S>(
    candidate: &'a T,
    specifications: &[S],
) -> Result<&'a T, DomainError>
where
    T: 'a,
    S: Specification<T> + fmt::Display,
{
    let mut result = SpecificationResult::new(candidate);

    for spec in specifications {
        if !spec.is_satisfied_by(candidate) {
            result.add_error(format!("Specification not satisfied: {}", spec));
        }
    }

    result.into_result()
}

/// Composite specification that requires both left and right to be satisfied
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

/// Composite specification that requires either left or right to be satisfied
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

/// Composite specification that negates the inner specification
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
    struct PositiveNumberSpec;

    impl Specification<i32> for EvenNumberSpec {
        fn is_satisfied_by(&self, candidate: &i32) -> bool {
            *candidate % 2 == 0
        }
    }

    impl fmt::Display for EvenNumberSpec {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "number must be even")
        }
    }

    impl Specification<i32> for PositiveNumberSpec {
        fn is_satisfied_by(&self, candidate: &i32) -> bool {
            *candidate > 0
        }
    }

    impl fmt::Display for PositiveNumberSpec {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "number must be positive")
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
        let spec = NotSpec::new(EvenNumberSpec);
        assert!(!spec.is_satisfied_by(&4));
        assert!(spec.is_satisfied_by(&3));
    }

    #[test]
    fn test_composite_specifications() {
        // Test with composite spec (AND) - both even AND positive
        let and_spec = EvenNumberSpec.and(PositiveNumberSpec);
        assert!(and_spec.is_satisfied_by(&4));
        assert!(!and_spec.is_satisfied_by(&-2)); // even but not positive
        assert!(!and_spec.is_satisfied_by(&3)); // positive but not even

        // Test with composite spec (OR) - even OR positive
        let or_spec = EvenNumberSpec.or(PositiveNumberSpec);
        assert!(or_spec.is_satisfied_by(&3)); // positive
        assert!(or_spec.is_satisfied_by(&-2)); // even
        assert!(!or_spec.is_satisfied_by(&-3)); // neither even nor positive

        // Test NOT specification
        let not_spec = PositiveNumberSpec.not();
        assert!(!not_spec.is_satisfied_by(&5)); // 5 is positive, so NOT positive should be false
        assert!(not_spec.is_satisfied_by(&-5)); // -5 is NOT positive, so NOT positive should be true
        assert!(not_spec.is_satisfied_by(&0)); // 0 is NOT positive, so NOT positive should be true
    }
}
