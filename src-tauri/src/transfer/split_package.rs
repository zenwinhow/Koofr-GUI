use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::AppError;

pub const MIN_PART_BYTES: u64 = 1024 * 1024;
pub const MAX_PART_BYTES: u64 = 4 * 1024 * 1024 * 1024;
pub const MANIFEST_NAME: &str = "manifest.json";
pub const RESTORE_CMD_NAME: &str = "restore.cmd";
pub const RESTORE_SH_NAME: &str = "restore.sh";
pub const SHA256SUMS_NAME: &str = "SHA256SUMS";
pub const README_NAME: &str = "README.txt";
const FORMAT_NAME: &str = "raw-binary-concatenation";
const PACKAGE_MARKER: &str = ".parts-";
const MAX_REMOTE_NAME_UNITS: usize = 255;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SplitPart {
    pub index: u32,
    pub size: u64,
    pub sha256: String,
}

impl SplitPart {
    pub fn new(index: u32, size: u64, sha256: String) -> Self {
        Self {
            index,
            size,
            sha256,
        }
    }

    pub fn file_name(&self) -> String {
        part_file_name(self.index)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SplitManifest {
    format: String,
    version: u8,
    pub file_name: String,
    pub total_bytes: u64,
    pub part_bytes: u64,
    pub file_sha256: String,
    pub parts: Vec<SplitPart>,
}

impl SplitManifest {
    pub fn new(
        file_name: String,
        total_bytes: u64,
        part_bytes: u64,
        file_sha256: String,
        parts: Vec<SplitPart>,
    ) -> Result<Self, AppError> {
        let manifest = Self {
            format: FORMAT_NAME.to_owned(),
            version: 1,
            file_name,
            total_bytes,
            part_bytes,
            file_sha256,
            parts,
        };
        manifest.validate()?;
        Ok(manifest)
    }

    pub fn validate(&self) -> Result<(), AppError> {
        let valid_name = !self.file_name.is_empty()
            && !self.file_name.contains(['/', '\0'])
            && self.file_name.encode_utf16().count() <= MAX_REMOTE_NAME_UNITS;
        if self.format != FORMAT_NAME
            || self.version != 1
            || !valid_name
            || self.total_bytes == 0
            || self.part_bytes == 0
            || !is_sha256(&self.file_sha256)
        {
            return Err(AppError::InvalidInput("split manifest"));
        }
        let mut covered = 0_u64;
        for (expected_index, part) in self.parts.iter().enumerate() {
            let index = u32::try_from(expected_index)
                .map_err(|_| AppError::InvalidInput("split part index"))?;
            let remaining = self.total_bytes.saturating_sub(covered);
            let expected_size = remaining.min(self.part_bytes);
            if part.index != index || part.size != expected_size || !is_sha256(&part.sha256) {
                return Err(AppError::InvalidInput("split part"));
            }
            covered = covered
                .checked_add(part.size)
                .ok_or(AppError::InvalidInput("split part size"))?;
        }
        if covered != self.total_bytes {
            return Err(AppError::IncompleteTransfer);
        }
        Ok(())
    }

    pub fn sha256sums(&self) -> String {
        let mut lines = self
            .parts
            .iter()
            .map(|part| format!("{} *{}", part.sha256, part.file_name()))
            .collect::<Vec<_>>();
        lines.push(format!("{} *{}", self.file_sha256, self.file_name));
        format!("{}\n", lines.join("\n"))
    }

    pub fn readme(&self) -> String {
        format!(
            "通用二进制分卷 / Raw binary split file\r\n\r\n原文件 / Original file: {}\r\n总大小 / Total bytes: {}\r\n每卷上限 / Part bytes: {}\r\n\r\nWindows（在此文件夹打开 CMD）：\r\n  copy /b /y part-*.bin \"<输出文件>\"\r\n  或 / or: restore.cmd \"<输出文件>\"\r\n  校验 / verify: certutil -hashfile \"<输出文件>\" SHA256\r\n\r\nLinux/macOS：\r\n  cat part-*.bin > \"<输出文件>\"\r\n  或 / or: sh restore.sh \"<输出文件>\"\r\n  Linux 校验 / verify: sha256sum \"<输出文件>\"\r\n  macOS 校验 / verify: shasum -a 256 \"<输出文件>\"\r\n\r\n分卷没有专有文件头，按文件名顺序直接拼接即可还原。完整文件的预期 SHA-256 位于 SHA256SUMS 最后一行和 manifest.json。\r\nThe parts have no proprietary headers. Concatenate part-*.bin in lexical order. The expected full-file SHA-256 is the final SHA256SUMS entry and is also stored in manifest.json.\r\n",
            self.file_name, self.total_bytes, self.part_bytes
        )
    }
}

pub fn package_directory_name(file_name: &str, transfer_id: Uuid) -> String {
    let suffix = format!("{PACKAGE_MARKER}{transfer_id}");
    let available = MAX_REMOTE_NAME_UNITS.saturating_sub(suffix.len());
    let mut base = String::new();
    let mut units = 0_usize;
    for character in file_name.chars() {
        let next = character.len_utf16();
        if units.saturating_add(next) > available {
            break;
        }
        base.push(character);
        units += next;
    }
    if base.is_empty() {
        base.push_str("file");
    }
    format!("{base}{suffix}")
}

pub fn validate_part_bytes(part_bytes: u64) -> Result<u64, AppError> {
    if !(MIN_PART_BYTES..=MAX_PART_BYTES).contains(&part_bytes) {
        return Err(AppError::InvalidInput("split part size"));
    }
    Ok(part_bytes)
}

pub fn part_file_name(index: u32) -> String {
    format!("part-{:010}.bin", u64::from(index) + 1)
}

pub const fn restore_cmd() -> &'static str {
    "@echo off\r\nsetlocal\r\nif \"%~1\"==\"\" (\r\n  echo Usage: restore.cmd output-file\r\n  exit /b 2\r\n)\r\ncopy /b /y part-*.bin \"%~1\" >nul\r\nif errorlevel 1 exit /b 1\r\necho Restored %~1\r\n"
}

pub const fn restore_sh() -> &'static str {
    "#!/bin/sh\nset -eu\nif [ \"$#\" -ne 1 ]; then\n  echo 'Usage: sh restore.sh output-file' >&2\n  exit 2\nfi\ncat part-*.bin > \"$1\"\necho \"Restored $1\"\n"
}

pub fn encode_sha256(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut encoded = String::with_capacity(bytes.len().saturating_mul(2));
    for byte in bytes {
        encoded.push(char::from(HEX[usize::from(byte >> 4)]));
        encoded.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    encoded
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}
