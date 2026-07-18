use reqwest::{StatusCode, header::HeaderValue};

use crate::error::AppError;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ResponseMode {
    Append,
    Restart,
}

pub fn response_mode(
    offset: u64,
    status: StatusCode,
    content_range: Option<&HeaderValue>,
    expected_size: u64,
) -> Result<ResponseMode, AppError> {
    if status == StatusCode::OK {
        return Ok(ResponseMode::Restart);
    }
    if status != StatusCode::PARTIAL_CONTENT {
        return Err(AppError::IncompleteTransfer);
    }
    let value = content_range
        .and_then(|header| header.to_str().ok())
        .ok_or(AppError::IncompleteTransfer)?;
    validate_content_range(value, offset, expected_size)?;
    Ok(ResponseMode::Append)
}

fn validate_content_range(value: &str, offset: u64, expected_size: u64) -> Result<(), AppError> {
    let value = value
        .strip_prefix("bytes ")
        .ok_or(AppError::IncompleteTransfer)?;
    let (range, total) = value.split_once('/').ok_or(AppError::IncompleteTransfer)?;
    let (start, end) = range.split_once('-').ok_or(AppError::IncompleteTransfer)?;
    let start = start
        .parse::<u64>()
        .map_err(|_| AppError::IncompleteTransfer)?;
    let end = end
        .parse::<u64>()
        .map_err(|_| AppError::IncompleteTransfer)?;
    let total = total
        .parse::<u64>()
        .map_err(|_| AppError::IncompleteTransfer)?;
    if start != offset || end < start || total != expected_size || end >= total {
        return Err(AppError::IncompleteTransfer);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use reqwest::{StatusCode, header::HeaderValue};

    use super::{ResponseMode, response_mode};

    #[test]
    fn appends_only_when_content_range_confirms_the_requested_offset() {
        let valid = HeaderValue::from_static("bytes 32-127/128");
        assert_eq!(
            response_mode(32, StatusCode::PARTIAL_CONTENT, Some(&valid), 128)
                .expect("accept valid range"),
            ResponseMode::Append
        );

        let wrong_start = HeaderValue::from_static("bytes 0-127/128");
        assert!(response_mode(32, StatusCode::PARTIAL_CONTENT, Some(&wrong_start), 128).is_err());
        assert_eq!(
            response_mode(32, StatusCode::OK, None, 128).expect("restart full response"),
            ResponseMode::Restart
        );
    }
}
