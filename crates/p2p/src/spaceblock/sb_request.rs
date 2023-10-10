use thiserror::Error;
use tokio::io::{AsyncRead, AsyncReadExt};

use crate::proto::{decode, encode};

use super::BlockSize;

/// TODO
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Range {
	/// Request the entire file
	Full,
	/// Partial range
	Partial(std::ops::Range<u64>),
}

impl Range {
	// TODO: Per field and proper error handling
	pub async fn from_stream(stream: &mut (impl AsyncRead + Unpin)) -> std::io::Result<Self> {
		match stream.read_u8().await.unwrap() {
			0 => Ok(Self::Full),
			1 => {
				let start = stream.read_u64_le().await.unwrap();
				let end = stream.read_u64_le().await.unwrap();
				Ok(Self::Partial(start..end))
			}
			_ => todo!(),
		}
	}

	pub fn to_bytes(&self) -> Vec<u8> {
		let mut buf = Vec::new();

		match self {
			Self::Full => buf.push(0),
			Self::Partial(range) => {
				buf.push(1);
				buf.extend_from_slice(&range.start.to_le_bytes());
				buf.extend_from_slice(&range.end.to_le_bytes());
			}
		}
		buf
	}
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpaceblockRequests {
	pub block_size: BlockSize,
	pub requests: Vec<SpaceblockRequest>,
}

#[derive(Debug, Error)]
pub enum SpaceblockRequestsError {
	#[error("SpaceblockRequestsError::InvalidLen({0})")]
	InvalidLen(std::io::Error),
	#[error("SpaceblockRequestsError::SpaceblockRequest({0:?})")]
	SpaceblockRequest(#[from] SpaceblockRequestError),
	#[error("SpaceblockRequestsError::BlockSize({0:?})")]
	BlockSize(std::io::Error),
}

impl SpaceblockRequests {
	pub async fn from_stream(
		stream: &mut (impl AsyncRead + Unpin),
	) -> Result<Self, SpaceblockRequestsError> {
		let block_size = BlockSize::from_stream(stream)
			.await
			.map_err(SpaceblockRequestsError::BlockSize)?;

		let size = stream
			// Max of 255 files in one request
			.read_u8()
			.await
			.map_err(SpaceblockRequestsError::InvalidLen)?;

		let mut requests = Vec::new();
		for i in 0..size {
			requests.push(SpaceblockRequest::from_stream(stream).await?);
		}

		Ok(Self {
			block_size,
			requests,
		})
	}

	pub fn to_bytes(&self) -> Vec<u8> {
		let Self {
			block_size,
			requests,
		} = self;
		if requests.len() > 255 {
			panic!("Can't Spacedrop more than 255 files at once!");
		}

		let mut buf = block_size.to_bytes().to_vec();
		buf.push(requests.len() as u8);
		for request in requests {
			buf.extend_from_slice(&request.to_bytes());
		}
		buf
	}
}

/// TODO
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpaceblockRequest {
	pub name: String,
	pub size: u64,
	// TODO: Include file permissions
	pub range: Range,
}

#[derive(Debug, Error)]
pub enum SpaceblockRequestError {
	#[error("SpaceblockRequestError::Name({0})")]
	Name(decode::Error),
	#[error("SpaceblockRequestError::Size({0})")]
	Size(std::io::Error),
}

impl SpaceblockRequest {
	pub async fn from_stream(
		stream: &mut (impl AsyncRead + Unpin),
	) -> Result<Self, SpaceblockRequestError> {
		let name = decode::string(stream)
			.await
			.map_err(SpaceblockRequestError::Name)?;

		let size = stream
			.read_u64_le()
			.await
			.map_err(SpaceblockRequestError::Size)?;

		Ok(Self {
			name,
			size,
			range: Range::from_stream(stream).await.unwrap(),
		})
	}

	pub fn to_bytes(&self) -> Vec<u8> {
		let Self { name, size, range } = self;
		let mut buf = Vec::new();

		encode::string(&mut buf, name);
		buf.extend_from_slice(&self.size.to_le_bytes());
		buf.extend_from_slice(&self.range.to_bytes());
		buf
	}
}

// TODO: This file is missing protocol unit tests
