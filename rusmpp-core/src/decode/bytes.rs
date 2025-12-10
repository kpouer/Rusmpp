use bytes::BytesMut;

use crate::decode::DecodeError;

pub trait Decode: Sized {
    /// Decode a value from a buffer.
    fn decode(src: &mut BytesMut) -> Result<Self, DecodeError>;
}

pub trait DecodeWithLength: Sized {
    /// Decode a value from a buffer, with a specified length
    fn decode(src: &mut BytesMut, length: usize) -> Result<Self, DecodeError>;
}

/// Everything that implements [`Decode`] also implements [`DecodeWithLength`] by ignoring the length.
impl<T: Decode> DecodeWithLength for T {
    fn decode(src: &mut BytesMut, _length: usize) -> Result<Self, DecodeError> {
        Decode::decode(src)
    }
}

pub trait DecodeWithKey: Sized {
    type Key;

    /// Decode a value from a buffer, using a key to determine the type.
    fn decode(key: Self::Key, src: &mut BytesMut, length: usize) -> Result<Self, DecodeError>;
}

pub trait DecodeWithKeyOptional: Sized {
    type Key;

    /// Decode an optional value from a buffer, using a key to determine the type.
    fn decode(
        key: Self::Key,
        src: &mut BytesMut,
        length: usize,
    ) -> Result<Option<Self>, DecodeError>;
}

#[doc(hidden)]
pub trait DecodeExt: Decode {
    /// Decode a vector of values from a buffer with a specified count.
    fn counted_decode(
        src: &mut BytesMut,
        count: usize,
    ) -> Result<alloc::vec::Vec<Self>, DecodeError> {
        (0..count).try_fold(alloc::vec::Vec::with_capacity(count), |mut vec, _| {
            Self::decode(src).map(|item| {
                vec.push(item);

                vec
            })
        })
    }

    /// Decode a value from a slice.
    ///
    /// If the length is 0, return `None`.
    fn length_checked_decode(
        src: &mut BytesMut,
        length: usize,
    ) -> Result<Option<Self>, DecodeError> {
        (length > 0)
            .then_some(())
            .map(|_| Self::decode(src))
            .transpose()
    }
}

impl<T: Decode> DecodeExt for T {}

#[doc(hidden)]
pub trait DecodeWithKeyExt: DecodeWithKey {
    /// Decode a value from a buffer, using a key to determine the type.
    ///
    /// If the length is 0, return `None`.
    fn optional_length_checked_decode(
        key: Self::Key,
        src: &mut BytesMut,
        length: usize,
    ) -> Result<Option<Self>, DecodeError> {
        (length > 0)
            .then_some(())
            .map(|_| Self::decode(key, src, length))
            .transpose()
    }
}

impl<T: DecodeWithKey> DecodeWithKeyExt for T {}

impl<T: Decode> DecodeWithLength for alloc::vec::Vec<T> {
    fn decode(src: &mut BytesMut, length: usize) -> Result<Self, DecodeError> {
        if length == 0 {
            return Ok(alloc::vec::Vec::new());
        }

        if length > src.len() {
            return Err(DecodeError::unexpected_eof());
        }

        let mut field = src.split_to(length);

        let mut vec = alloc::vec::Vec::new();

        while !field.is_empty() {
            let before = field.len();

            let item = T::decode(&mut field)?;

            let consumed = before - field.len();

            if consumed == 0 {
                return Err(DecodeError::unexpected_eof());
            }

            vec.push(item);
        }

        Ok(vec)
    }
}

#[cfg(test)]
mod tests {
    use alloc::vec::Vec;

    use crate::{
        decode::{COctetStringDecodeError, DecodeErrorKind},
        types::owned::{COctetString, EmptyOrFullCOctetString},
    };

    use super::*;

    #[test]
    fn counted() {
        // Count is 0
        let mut buf = BytesMut::from(&[0, 1, 2][..]);

        let values = u8::counted_decode(&mut buf, 0).unwrap();

        assert_eq!(values.len(), 0);
        assert_eq!(&buf[..], &[0, 1, 2]);
        assert_eq!(values, Vec::<u8>::new());

        // Count is more than the buffer
        let mut buf = BytesMut::from(&[0, 1, 2][..]);

        let error = u8::counted_decode(&mut buf, 5).unwrap_err();
        assert!(matches!(error.kind(), DecodeErrorKind::UnexpectedEof));

        // Count is within the buffer
        let mut buf = BytesMut::from(&[0, 1, 2, 3, 4, 5, 6, 7, 8, 9][..]);

        let values = u8::counted_decode(&mut buf, 10).unwrap();

        assert_eq!(values.len(), 10);
        assert!(buf.is_empty());
        assert_eq!(values, alloc::vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);

        let mut buf =
            BytesMut::from(&[0, 0, 0, 1, 0, 2, 0, 3, 0, 4, 0, 5, 0, 6, 0, 7, 0, 8, 0, 9][..]);

        let values = u16::counted_decode(&mut buf, 10).unwrap();

        assert_eq!(values.len(), 10);
        assert!(buf.is_empty());
        assert_eq!(values, alloc::vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);

        let mut buf = BytesMut::from(
            &[
                0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 2, 0, 0, 0, 3, 0, 0, 0, 4, 0, 0, 0, 5, 0, 0, 0, 6,
                0, 0, 0, 7, 0, 0, 0, 8, 0, 0, 0, 9,
            ][..],
        );

        // Actually 10 values, 12 will break
        let error = u32::counted_decode(&mut buf, 12).unwrap_err();

        assert!(matches!(error.kind(), DecodeErrorKind::UnexpectedEof));

        let mut buf = BytesMut::from(&b"Hello\0World\0"[..]);

        let values = COctetString::<1, 6>::counted_decode(&mut buf, 2).unwrap();

        assert!(&buf[..].is_empty());
        assert_eq!(
            values,
            alloc::vec![
                COctetString::<1, 6>::from_static_slice(b"Hello\0").unwrap(),
                COctetString::<1, 6>::from_static_slice(b"World\0").unwrap(),
            ]
        );

        let mut buf = BytesMut::from(&b"Hello\0World\0"[..]);

        let values = EmptyOrFullCOctetString::<6>::counted_decode(&mut buf, 2).unwrap();

        assert!(&buf[..].is_empty());
        assert_eq!(
            values,
            alloc::vec![
                EmptyOrFullCOctetString::<6>::from_static_slice(b"Hello\0").unwrap(),
                EmptyOrFullCOctetString::<6>::from_static_slice(b"World\0").unwrap(),
            ]
        );

        let mut buf = BytesMut::from(&b"Hello\0World\0Hi"[..]);

        let error = COctetString::<1, 6>::counted_decode(&mut buf, 3).unwrap_err();

        assert!(matches!(
            error.kind(),
            DecodeErrorKind::COctetStringDecodeError(COctetStringDecodeError::NotNullTerminated)
        ));

        // Remaining bytes
        let mut buf = BytesMut::from(&[0, 1, 2, 3, 4, 5, 6, 7, 8, 9][..]);

        let values = u8::counted_decode(&mut buf, 5).unwrap();

        assert_eq!(&buf[..], &[5, 6, 7, 8, 9]);
        assert_eq!(values, alloc::vec![0, 1, 2, 3, 4]);

        let mut buf =
            BytesMut::from(&[0, 0, 0, 1, 0, 2, 0, 3, 0, 4, 0, 5, 0, 6, 0, 7, 0, 8, 0, 9][..]);

        let values = u16::counted_decode(&mut buf, 5).unwrap();

        assert_eq!(&buf[..], &[0, 5, 0, 6, 0, 7, 0, 8, 0, 9]);
        assert_eq!(values, alloc::vec![0, 1, 2, 3, 4]);
    }

    #[test]
    fn decode_with_length_vec() {
        // Length is 0
        let mut buf = BytesMut::from(&[0, 1, 2][..]);

        let values = Vec::<u8>::decode(&mut buf, 0).unwrap();

        assert_eq!(&buf[..], &[0, 1, 2]);
        assert_eq!(values, Vec::<u8>::new());

        // Length is bigger than the buffer
        let mut buf = BytesMut::from(&[0, 1, 2][..]);

        let error = Vec::<u8>::decode(&mut buf, 5).unwrap_err();

        assert!(matches!(error.kind(), DecodeErrorKind::UnexpectedEof));

        // Length is within the buffer
        let mut buf = BytesMut::from(&[0, 1, 2, 3, 4, 5, 6, 7, 8, 9][..]);

        let values = Vec::<u8>::decode(&mut buf, 10).unwrap();

        assert!(buf.is_empty());
        assert_eq!(values, alloc::vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);

        let mut buf =
            BytesMut::from(&[0, 0, 0, 1, 0, 2, 0, 3, 0, 4, 0, 5, 0, 6, 0, 7, 0, 8, 0, 9][..]);

        let values = Vec::<u16>::decode(&mut buf, 20).unwrap();

        assert!(buf.is_empty());
        assert_eq!(values, alloc::vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);

        let mut buf = BytesMut::from(
            &[
                0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 2, 0, 0, 0, 3, 0, 0, 0, 4, 0, 0, 0, 5, 0, 0, 0, 6,
                0, 0, 0, 7, 0, 0, 0, 8, 0, 0, 0, 9,
            ][..],
        );

        // Actually 40 bytes, 50 will break
        let error = Vec::<u32>::decode(&mut buf, 50).unwrap_err();

        assert!(matches!(error.kind(), DecodeErrorKind::UnexpectedEof));

        let mut buf = BytesMut::from(&b"Hello\0World\0"[..]);

        let values = Vec::<COctetString<1, 6>>::decode(&mut buf, 12).unwrap();

        assert!(buf.is_empty());
        assert_eq!(
            values,
            alloc::vec![
                COctetString::<1, 6>::from_static_slice(b"Hello\0").unwrap(),
                COctetString::<1, 6>::from_static_slice(b"World\0").unwrap(),
            ]
        );

        let mut buf = BytesMut::from(&b"Hello\0World\0"[..]);

        let values = Vec::<EmptyOrFullCOctetString<6>>::decode(&mut buf, 12).unwrap();

        assert!(buf.is_empty());
        assert_eq!(
            values,
            alloc::vec![
                EmptyOrFullCOctetString::<6>::from_static_slice(b"Hello\0").unwrap(),
                EmptyOrFullCOctetString::<6>::from_static_slice(b"World\0").unwrap(),
            ]
        );

        let mut buf = BytesMut::from(&b"Hello\0World\0Hi"[..]);

        // This will try to decode 11 bytes b"Hello\0World"
        let error = Vec::<COctetString<1, 6>>::decode(&mut buf, 11).unwrap_err();

        assert!(matches!(
            error.kind(),
            DecodeErrorKind::COctetStringDecodeError(COctetStringDecodeError::NotNullTerminated)
        ));

        // Remaining bytes
        let mut buf = BytesMut::from(&[0, 1, 2, 3, 4, 5, 6, 7, 8, 9][..]);

        let values = Vec::<u8>::decode(&mut buf, 5).unwrap();

        assert_eq!(&buf[..], &[5, 6, 7, 8, 9]);
        assert_eq!(values, alloc::vec![0, 1, 2, 3, 4]);

        let mut buf =
            BytesMut::from(&[0, 0, 0, 1, 0, 2, 0, 3, 0, 4, 0, 5, 0, 6, 0, 7, 0, 8, 0, 9][..]);

        let values = Vec::<u16>::decode(&mut buf, 10).unwrap();

        assert_eq!(&buf[..], &[0, 5, 0, 6, 0, 7, 0, 8, 0, 9]);
        assert_eq!(values, alloc::vec![0, 1, 2, 3, 4]);
    }
}
