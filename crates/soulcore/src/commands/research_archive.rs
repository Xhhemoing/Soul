//! Standard stored ZIP, not an application backup or a cryptographic envelope.
use soul_store::research_retention::ResearchDataset;

const LIMIT: usize = 1024 * 1024;
const NOTICE: &str = "明文独立副本；原库遗忘不会删除本文件；不能用于恢复 Soul。";
const README: &str = "明文独立副本；原库遗忘不会删除本文件；不能用于恢复 Soul。\n\
records.jsonl: one self activity event-count bucket per UTC hour, not duration.\n\
metadata.json: scope, field names, preparation-time counts and program version.\n\
Excluded: body, identifiers, application names, duration and profile.\n\
Missing observations are not zero activity. No raw data, keys or application state.\n\
Read with Python 3: python scripts/read-research-retention.py ARCHIVE.zip\n";

pub(super) fn encode(
    dataset: &ResearchDataset,
    prepared_at: &str,
) -> Result<Vec<u8>, &'static str> {
    let mut records = Vec::new();
    for row in &dataset.rows {
        if row.event_kind.len() > LIMIT || row.time_bucket_utc.len() > LIMIT {
            return Err("RESEARCH_ARCHIVE_TOO_LARGE");
        }
        serde_json::to_writer(&mut records, row).map_err(|_| "RESEARCH_ARCHIVE_ENCODING")?;
        records.push(b'\n');
        if records.len() > LIMIT {
            return Err("RESEARCH_ARCHIVE_TOO_LARGE");
        }
    }
    if prepared_at.len() > LIMIT
        || dataset.scope.from_day.len() > LIMIT
        || dataset.scope.to_day.len() > LIMIT
    {
        return Err("RESEARCH_ARCHIVE_TOO_LARGE");
    }
    let metadata = serde_json::to_vec(&serde_json::json!({
        "format_version": 1,
        "program_version": env!("CARGO_PKG_VERSION"),
        "prepared_at_utc": prepared_at,
        "scope": dataset.scope,
        "counts": dataset.counts,
        "row_count": dataset.rows.len(),
        "fields": ["event_kind", "time_bucket_utc", "aggregate_count"],
        "purpose": "research",
        "population": "self",
        "granularity": "hour",
        "excluded_values": ["body", "identifier", "app", "duration", "profile"],
        "counts_timing": "preparation",
        "notice": NOTICE,
    }))
    .map_err(|_| "RESEARCH_ARCHIVE_ENCODING")?;
    stored_zip(&[
        ("records.jsonl", &records),
        ("metadata.json", &metadata),
        ("README.txt", README.as_bytes()),
    ])
}

fn word(output: &mut Vec<u8>, value: u16) {
    output.extend_from_slice(&value.to_le_bytes());
}

fn dword(output: &mut Vec<u8>, value: u32) {
    output.extend_from_slice(&value.to_le_bytes());
}

/// Required ZIP CRC32, not a new source hash or integrity approval mechanism.
fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = u32::MAX;
    for byte in bytes {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            crc = (crc >> 1) ^ (0xedb8_8320 & (0u32.wrapping_sub(crc & 1)));
        }
    }
    !crc
}

fn stored_zip(entries: &[(&str, &[u8])]) -> Result<Vec<u8>, &'static str> {
    // Three local headers, central directory headers, and the final record.
    // The complete package, including metadata and overhead, must fit.
    let size = entries
        .iter()
        .try_fold(22usize, |size, (name, data)| {
            size.checked_add(30 + 46 + name.len() * 2)?
                .checked_add(data.len())
        })
        .ok_or("RESEARCH_ARCHIVE_TOO_LARGE")?;
    if size > LIMIT {
        return Err("RESEARCH_ARCHIVE_TOO_LARGE");
    }
    let mut output = Vec::with_capacity(size);
    let mut central = Vec::new();
    for (name, data) in entries {
        let offset = output.len() as u32;
        let crc = crc32(data);
        dword(&mut output, 0x0403_4b50);
        word(&mut output, 20); // version needed: ZIP 2.0
        word(&mut output, 0x0800); // UTF-8 names, no encryption or descriptor
        word(&mut output, 0); // stored
        word(&mut output, 0); // 00:00:00
        word(&mut output, 33); // 1980-01-01; observation time is in metadata
        dword(&mut output, crc);
        dword(&mut output, data.len() as u32);
        dword(&mut output, data.len() as u32);
        word(&mut output, name.len() as u16);
        word(&mut output, 0); // no extra field
        output.extend_from_slice(name.as_bytes());
        output.extend_from_slice(data);

        dword(&mut central, 0x0201_4b50);
        word(&mut central, 20); // made by DOS/ZIP 2.0
        word(&mut central, 20);
        word(&mut central, 0x0800);
        word(&mut central, 0);
        word(&mut central, 0);
        word(&mut central, 33);
        dword(&mut central, crc);
        dword(&mut central, data.len() as u32);
        dword(&mut central, data.len() as u32);
        word(&mut central, name.len() as u16);
        word(&mut central, 0); // extra length
        word(&mut central, 0); // comment length
        word(&mut central, 0); // starting disk
        word(&mut central, 0); // internal attributes
        dword(&mut central, 0); // external attributes
        dword(&mut central, offset);
        central.extend_from_slice(name.as_bytes());
    }
    let central_offset = output.len() as u32;
    output.extend_from_slice(&central);
    dword(&mut output, 0x0605_4b50);
    word(&mut output, 0);
    word(&mut output, 0);
    word(&mut output, entries.len() as u16);
    word(&mut output, entries.len() as u16);
    dword(&mut output, central.len() as u32);
    dword(&mut output, central_offset);
    word(&mut output, 0);
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use soul_store::research_retention::{ActivityBucket, ResearchCounts, ResearchScope};

    fn dataset() -> ResearchDataset {
        ResearchDataset {
            scope: ResearchScope {
                from_day: "2024-02-29".into(),
                to_day: "2024-03-01".into(),
            },
            rows: vec![ActivityBucket {
                event_kind: "app.foreground".into(),
                time_bucket_utc: "2024-02-29T09:00:00Z".into(),
                aggregate_count: 2,
            }],
            counts: ResearchCounts {
                candidate_events: 2,
                included_events: 2,
                ..Default::default()
            },
        }
    }

    #[test]
    fn small_dataset_is_encoded_as_three_stored_zip_entries() {
        let bytes = encode(&dataset(), "2024-03-02T01:02:03Z").unwrap();
        assert_eq!(&bytes[..4], b"PK\x03\x04");
        let mut offset = 0;
        let mut names = Vec::new();
        while bytes[offset..].starts_with(b"PK\x03\x04") {
            assert_eq!(&bytes[offset + 8..offset + 10], &[0, 0]);
            let size =
                u32::from_le_bytes(bytes[offset + 18..offset + 22].try_into().unwrap()) as usize;
            let name_len =
                u16::from_le_bytes(bytes[offset + 26..offset + 28].try_into().unwrap()) as usize;
            let extra_len =
                u16::from_le_bytes(bytes[offset + 28..offset + 30].try_into().unwrap()) as usize;
            names.push(std::str::from_utf8(&bytes[offset + 30..offset + 30 + name_len]).unwrap());
            offset += 30 + name_len + extra_len + size;
        }
        assert_eq!(names, ["records.jsonl", "metadata.json", "README.txt"]);
        assert!(bytes[offset..].starts_with(b"PK\x01\x02"));
        assert!(bytes[bytes.len() - 22..].starts_with(b"PK\x05\x06"));
    }

    #[test]
    fn complete_archive_over_one_mib_is_refused_without_truncation() {
        let mut data = dataset();
        data.rows[0].event_kind = "x".repeat(1024 * 1024);
        assert_eq!(
            encode(&data, "2024-03-02T01:02:03Z").unwrap_err(),
            "RESEARCH_ARCHIVE_TOO_LARGE"
        );
    }

    #[test]
    fn zip_crc_matches_the_standard_check_vector() {
        assert_eq!(crc32(b"123456789"), 0xcbf4_3926);
        assert_eq!(crc32(b""), 0);
    }

    #[test]
    fn one_mib_limit_counts_headers_and_the_central_directory() {
        // 22 + 3*(30+46) + 2*(13+13+10) = 322 bytes of ZIP overhead.
        let data = vec![b'x'; 1_048_576 - 322];
        let package = stored_zip(&[
            ("records.jsonl", &data),
            ("metadata.json", &[]),
            ("README.txt", &[]),
        ])
        .unwrap();
        assert_eq!(package.len(), 1_048_576);
        let excessive = vec![b'x'; 1_048_576 - 321];
        assert_eq!(
            stored_zip(&[
                ("records.jsonl", &excessive),
                ("metadata.json", &[]),
                ("README.txt", &[]),
            ])
            .unwrap_err(),
            "RESEARCH_ARCHIVE_TOO_LARGE"
        );
    }
}
