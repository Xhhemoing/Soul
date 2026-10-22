"""Read a Soul research archive without extracting files or accessing Soul."""
import datetime
import io
import json
import re
import struct
import sys
import zipfile

MAX_BYTES = 1024 * 1024
ENTRIES = {"records.jsonl", "metadata.json", "README.txt"}
FIELDS = ["event_kind", "time_bucket_utc", "aggregate_count"]
COUNTS = {"candidate_events", "included_events", "excluded_policy_events",
          "excluded_unavailable_events", "excluded_time_events", "out_of_range_events"}
META_FIELDS = {"format_version", "program_version", "prepared_at_utc", "scope", "counts",
               "row_count", "fields", "purpose", "population", "granularity",
               "excluded_values", "counts_timing", "notice"}
NOTICE = "明文独立副本；原库遗忘不会删除本文件；不能用于恢复 Soul。"


class InvalidArchive(ValueError):
    pass


def require(condition):
    if not condition:
        raise InvalidArchive("invalid research archive")


def exact_object(value, fields):
    require(type(value) is dict and set(value) == set(fields))


def integer(value, minimum=0):
    require(type(value) is int and minimum <= value <= 100_000)


def day(value):
    require(type(value) is str and re.fullmatch(r"[0-9]{4}-[0-9]{2}-[0-9]{2}", value))
    try:
        return datetime.date.fromisoformat(value)
    except ValueError:
        raise InvalidArchive("invalid date") from None


def utc(value):
    require(type(value) is str and re.fullmatch(
        r"[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}Z", value))
    try:
        return datetime.datetime.strptime(value, "%Y-%m-%dT%H:%M:%SZ")
    except ValueError:
        raise InvalidArchive("invalid UTC time") from None


def unique_object(pairs):
    result = {}
    for key, value in pairs:
        require(key not in result)
        result[key] = value
    return result


def parse_json(data):
    return json.loads(data, object_pairs_hook=unique_object,
                      parse_constant=lambda _: require(False))


def inspect(payload):
    with zipfile.ZipFile(io.BytesIO(payload)) as archive:
        entries = archive.infolist()
        require(len(entries) == 3 and {entry.filename for entry in entries} == ENTRIES)
        require(sum(entry.file_size for entry in entries) <= MAX_BYTES)
        for entry in entries:
            require(entry.compress_type == zipfile.ZIP_STORED and not entry.flag_bits & 1)
            require(0 <= entry.file_size <= MAX_BYTES and entry.compress_size == entry.file_size)
        # zipfile verifies CRC and local headers; no extraction or file writes.
        contents = {entry.filename: archive.read(entry).decode("utf-8") for entry in entries}
    require(NOTICE in contents["README.txt"])
    metadata = parse_json(contents["metadata.json"])
    exact_object(metadata, META_FIELDS)
    require(type(metadata["format_version"]) is int and metadata["format_version"] == 1)
    require(type(metadata["program_version"]) is str and re.fullmatch(
        r"[0-9]+\.[0-9]+\.[0-9]+(?:[-+][A-Za-z0-9.+-]+)?", metadata["program_version"]))
    utc(metadata["prepared_at_utc"])
    exact_object(metadata["scope"], {"from_day", "to_day"})
    start = day(metadata["scope"]["from_day"])
    end = day(metadata["scope"]["to_day"])
    require(start < end)
    require(metadata["fields"] == FIELDS and metadata["purpose"] == "research"
            and metadata["population"] == "self" and metadata["granularity"] == "hour"
            and metadata["excluded_values"] == ["body", "identifier", "app", "duration", "profile"]
            and metadata["counts_timing"] == "preparation" and metadata["notice"] == NOTICE)
    counts = metadata["counts"]
    exact_object(counts, COUNTS)
    for value in counts.values():
        integer(value)
    integer(metadata["row_count"])
    require(counts["candidate_events"] == sum(value for key, value in counts.items()
                                             if key != "candidate_events"))
    records = contents["records.jsonl"]
    require(not records or records.endswith("\n"))
    lines = records.splitlines()
    require(len(lines) == metadata["row_count"])
    previous = ""
    included = 0
    for line in lines:
        row = parse_json(line)
        exact_object(row, FIELDS)
        require(row["event_kind"] == "app.foreground")
        moment = utc(row["time_bucket_utc"])
        require(moment.minute == 0 and moment.second == 0 and start <= moment.date() < end)
        require(row["time_bucket_utc"] > previous)
        previous = row["time_bucket_utc"]
        integer(row["aggregate_count"], minimum=1)
        included += row["aggregate_count"]
    require(included == counts["included_events"])
    return {"format_version": 1, "row_count": len(lines), "included_events": included,
            "candidate_events": counts["candidate_events"],
            "excluded_events": counts["candidate_events"] - included}


def read_archive(path):
    try:
        with open(path, "rb") as handle:
            payload = handle.read(MAX_BYTES + 1)
        require(len(payload) <= MAX_BYTES)
        return inspect(payload)
    except (OSError, ValueError, TypeError, KeyError, UnicodeError, RuntimeError,
            zipfile.BadZipFile, NotImplementedError, OverflowError, RecursionError,
            EOFError, struct.error):
        # Neither stored values nor filesystem paths are part of the error API.
        raise InvalidArchive("invalid research archive") from None


def main():
    try:
        require(len(sys.argv) == 2)
        result = read_archive(sys.argv[1])
    except InvalidArchive:
        print("Research archive rejected.", file=sys.stderr)
        return 2
    print(json.dumps(result, ensure_ascii=True, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
