//! Flatten an iterator of results of iterators...
//! ...yeah, it does sound a bit confusing, does it not?
//!
//! The main intended use of this trait is to build an iterator of
//! [`Result`]-wrapped options, and be able to easily skip [`None`]
//! values without e.g. storing the whole collection into a vector
//! using `.collect()?` and then running `.flatten()` onto that.
//!
//! ```not really a doc test since this is a private module
//! # use std::error::Error;
//! # use fnmatch_regex::try_flatten::TryFlatten;
//!
//! const OPT_OK: [Result<Option<&str>, &str>; 3] =
//!     [Ok(Some("hello")), Ok(None), Ok(Some("goodbye"))];
//!
//! const OPT_ERR: [Result<Option<&str>, &str>; 6] = [
//!     Ok(Some("hello")),
//!     Ok(None),
//!     Ok(Some("goodbye")),
//!     Ok(None),
//!     Err("oof"),
//!     Ok(Some("never")),
//! ];
//!
//! # fn main() -> Result<(), Box<dyn Error>> {
//! let data: Vec<&str> = OPT_OK.into_iter().try_flatten().collect::<Result<_, _>>()?;
//! println!("{} results collected successfully");
//! let res: Result<Vec<&str>> = OPT_ERR.into_iter().try_flatten().collect::<Result<_, _>>();
//! println!("This should be an error: {:?}", res);
//! # Ok(())
//! # }
//! ```

use std::iter;

/// An implementation of [`try_flatten()`].
pub struct TryFlattenImpl<T, E, IRI, II, I>
where
    IRI: Iterator<Item = Result<II, E>>,
    II: IntoIterator<Item = T, IntoIter = I>,
    I: Iterator<Item = T>,
{
    /// The iterator to wrap.
    it: IRI,
    /// Can we run .next() on .inner?
    ready: bool,
    /// The iterator we fetched from the iterator to wrap.
    inner: Box<dyn Iterator<Item = T>>,
}

impl<T, E, IRI, II, I> Iterator for TryFlattenImpl<T, E, IRI, II, I>
where
    IRI: Iterator<Item = Result<II, E>>,
    II: IntoIterator<Item = T, IntoIter = I>,
    I: Iterator<Item = T> + 'static,
    T: 'static,
{
    type Item = Result<T, E>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.ready {
            match self.inner.next() {
                Some(value) => Some(Ok(value)),
                None => {
                    self.ready = false;
                    self.next()
                }
            }
        } else {
            self.it.next().and_then(|res| match res {
                Ok(inner) => {
                    self.inner = Box::new(inner.into_iter());
                    self.ready = true;
                    self.next()
                }
                Err(err) => Some(Err(err)),
            })
        }
    }
}

/// Flatten an iterator of Result-wrapped iterators.
pub trait TryFlatten<T, E, IRI, II, I>: Iterator
where
    IRI: Iterator<Item = Result<II, E>>,
    II: IntoIterator<Item = T, IntoIter = I>,
    I: Iterator<Item = T>,
{
    /// Wrap an iterator, fail on errors, skip None values.
    fn try_flatten(self) -> TryFlattenImpl<T, E, IRI, II, I>;
}

impl<T, E, IRI, II, I> TryFlatten<T, E, IRI, II, I> for IRI
where
    IRI: Iterator<Item = Result<II, E>>,
    II: IntoIterator<Item = T, IntoIter = I>,
    I: Iterator<Item = T>,
    T: 'static,
{
    fn try_flatten(self) -> TryFlattenImpl<T, E, IRI, II, I>
    where
        IRI: Iterator<Item = Result<II, E>>,
        II: IntoIterator<Item = T, IntoIter = I>,
        I: Iterator<Item = T>,
    {
        TryFlattenImpl {
            it: self,
            ready: false,
            inner: Box::new(iter::empty()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::TryFlatten;

    const OPT_OK: [Result<Option<&str>, &str>; 3] =
        [Ok(Some("hello")), Ok(None), Ok(Some("goodbye"))];
    const OPT_ERR: [Result<Option<&str>, &str>; 6] = [
        Ok(Some("hello")),
        Ok(None),
        Ok(Some("goodbye")),
        Ok(None),
        Err("oof"),
        Ok(Some("never")),
    ];

    #[test]
    fn test_ok_option() {
        let mut it = OPT_OK.into_iter().try_flatten();
        assert_eq!(it.next(), Some(Ok("hello")));
        assert_eq!(it.next(), Some(Ok("goodbye")));
        assert_eq!(it.next(), None)
    }

    #[test]
    fn test_err_option() {
        let mut it = OPT_ERR.into_iter().try_flatten();
        assert_eq!(it.next(), Some(Ok("hello")));
        assert_eq!(it.next(), Some(Ok("goodbye")));
        assert_eq!(it.next(), Some(Err("oof")));
        assert_eq!(it.next(), Some(Ok("never")));
        assert_eq!(it.next(), None)
    }

    #[test]
    fn test_ok_option_collect() {
        let res: Result<Vec<_>, _> = OPT_OK.into_iter().try_flatten().collect();
        assert_eq!(res, Ok(vec!["hello", "goodbye"]));
    }

    #[test]
    fn test_err_option_collect() {
        let res: Result<Vec<_>, _> = OPT_ERR.into_iter().try_flatten().collect();
        assert_eq!(res, Err("oof"));
    }

    #[derive(Debug, Default)]
    struct LazyTest {
        count: usize,
    }

    impl Iterator for LazyTest {
        type Item = Result<Vec<u32>, String>;

        fn next(&mut self) -> Option<Self::Item> {
            self.count += 1;
            match self.count {
                1 => Some(Ok(vec![1, 2])),
                2 => Some(Ok(vec![])),
                3 => Some(Ok(vec![3])),
                4 => Some(Err("oof".to_owned())),
                _ => panic!("how did we get here?"),
            }
        }
    }

    #[test]
    fn test_lazy() {
        let mut it_values = LazyTest::default().try_flatten();
        assert_eq!(it_values.next(), Some(Ok(1)));
        assert_eq!(it_values.next(), Some(Ok(2)));
        assert_eq!(it_values.next(), Some(Ok(3)));
        assert_eq!(it_values.next(), Some(Err("oof".to_owned())));

        let it_collect = LazyTest::default().try_flatten();
        let res = it_collect.collect::<Result<Vec<_>, _>>();
        assert_eq!(res, Err("oof".to_owned()));
    }
}
