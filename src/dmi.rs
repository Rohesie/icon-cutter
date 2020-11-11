pub mod chunk;
pub mod crc;
pub mod error;
pub mod icon;
pub mod ztxt;

use std::convert::TryFrom;
use std::io::prelude::*;

/// The PNG magic header
pub const MAGIC: [u8; 8] = [137, 80, 78, 71, 13, 10, 26, 10];

#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct RawDmi {
	pub header: [u8; 8],
	pub chunk_ihdr: chunk::RawGenericChunk,
	pub chunk_ztxt: Option<ztxt::RawZtxtChunk>,
	pub chunk_idat: chunk::RawGenericChunk,
	pub chunk_iend: chunk::RawGenericChunk,
	pub other_chunks: Vec<chunk::RawGenericChunk>,
}

impl RawDmi {
	pub fn load<R: Read>(mut reader: R) -> Result<RawDmi, error::DmiError> {
		let mut dmi_bytes = Vec::new();
		reader.read_to_end(&mut dmi_bytes)?;
		// 8 bytes for the PNG file signature.
		// 12 + 13 bytes for the IHDR chunk.
		// 12 for the IDAT chunk.
		// 12 + 3 for the zTXt chunk.
		// 12 for the IEND chunk.

		// Total minimum size for a DMI file: 72 bytes.

		if dmi_bytes.len() < 72 {
			return Err(error::DmiError::Generic(format!("Failed to load DMI. Supplied reader contained size of {} bytes, lower than the required 72.", dmi_bytes.len())));
		};

		let header = &dmi_bytes[0..8];
		if dmi_bytes[0..8] != MAGIC {
			return Err(error::DmiError::Generic(format!(
				"PNG header mismatch (expected {:#?}, found {:#?})",
				MAGIC, header
			)));
		};
		let header = MAGIC;
		let mut chunk_ihdr = None;
		let mut chunk_ztxt = None;
		let mut chunk_idat = None;
		let chunk_iend;
		let mut other_chunks = vec![];

		// Index starts after the PNG header.
		let mut index = 8;

		loop {
			if index + 12 > dmi_bytes.len() {
				return Err(error::DmiError::Generic(
					"Failed to load DMI. Buffer end reached without finding an IEND chunk."
						.to_string(),
				));
			}

			let chunk_data_length = u32::from_be_bytes([
				dmi_bytes[index],
				dmi_bytes[index + 1],
				dmi_bytes[index + 2],
				dmi_bytes[index + 3],
			]) as usize;

			// 12 minimum necessary bytes from the chunk plus the data length.
			let chunk_bytes = dmi_bytes[index..(index + 12 + chunk_data_length)].to_vec();
			let raw_chunk = chunk::RawGenericChunk::load(&mut &*chunk_bytes)?;
			index += 12 + chunk_data_length;

			match &raw_chunk.chunk_type {
				b"IHDR" => chunk_ihdr = Some(raw_chunk),
				b"zTXt" => chunk_ztxt = Some(ztxt::RawZtxtChunk::try_from(raw_chunk)?),
				b"IDAT" => chunk_idat = Some(raw_chunk),
				b"IEND" => {
					chunk_iend = Some(raw_chunk);
					break;
				}
				_ => other_chunks.push(raw_chunk),
			}
		}
		if chunk_ihdr == None || chunk_idat == None {
			return Err(error::DmiError::Generic(format!("Failed to load DMI. Buffer end reached without finding a necessary chunk.\nIHDR: {:#?}\nIDAT: {:#?}", chunk_ihdr, chunk_idat)));
		};
		let chunk_ihdr = chunk_ihdr.unwrap();
		let chunk_idat = chunk_idat.unwrap();
		let chunk_iend = chunk_iend.unwrap();

		Ok(RawDmi {
			header,
			chunk_ihdr,
			chunk_ztxt,
			chunk_idat,
			chunk_iend,
			other_chunks,
		})
	}

	pub fn save<W: Write>(&self, mut writter: &mut W) -> Result<usize, error::DmiError> {
		let bytes_written = writter.write(&self.header)?;
		let mut total_bytes_written = bytes_written;
		if bytes_written < 8 {
			return Err(error::DmiError::Generic(format!(
				"Failed to save DMI. Buffer unable to hold the data, only {} bytes written.",
				total_bytes_written
			)));
		};

		let bytes_written = self.chunk_ihdr.save(&mut writter)?;
		total_bytes_written += bytes_written;
		if bytes_written < u32::from_be_bytes(self.chunk_ihdr.data_length) as usize + 12 {
			return Err(error::DmiError::Generic(format!(
				"Failed to save DMI. Buffer unable to hold the data, only {} bytes written.",
				total_bytes_written
			)));
		};

		match &self.chunk_ztxt {
			Some(chunk_ztxt) => {
				let bytes_written = chunk_ztxt.save(&mut writter)?;
				total_bytes_written += bytes_written;
				if bytes_written < u32::from_be_bytes(chunk_ztxt.data_length) as usize + 12 {
					return Err(error::DmiError::Generic(format!("Failed to save DMI. Buffer unable to hold the data, only {} bytes written.", total_bytes_written)));
				};
			}
			None => (),
		};

		let bytes_written = self.chunk_idat.save(&mut writter)?;
		total_bytes_written += bytes_written;
		if bytes_written < u32::from_be_bytes(self.chunk_idat.data_length) as usize + 12 {
			return Err(error::DmiError::Generic(format!(
				"Failed to save DMI. Buffer unable to hold the data, only {} bytes written.",
				total_bytes_written
			)));
		};

		let bytes_written = self.chunk_iend.save(&mut writter)?;
		total_bytes_written += bytes_written;
		if bytes_written < u32::from_be_bytes(self.chunk_iend.data_length) as usize + 12 {
			return Err(error::DmiError::Generic(format!(
				"Failed to save DMI. Buffer unable to hold the data, only {} bytes written.",
				total_bytes_written
			)));
		};

		Ok(total_bytes_written)
	}

}
