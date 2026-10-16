//! E1 (S3/v1 physical carrier): bounded reader for Zig 0.16.
//! Pure carrier preflight only; width and payload are opaque transport data.
//! It is NOT .sens T5, .senc F3C, E2/E3, or SENS semantic authority.
//! This heapless/OS-free reader does NOT verify D2 or optimal-DP canonicality.
//! A physical success must NOT be used as E1 executable/admission PASS.
const std = @import("std");
const fields = @import("fields.zig");

pub const Word = struct { width: u8, bits: u16 };
pub const MAX_WORDS: usize = 4096;
pub const MAX_BLOCK_WORDS: usize = 128;
pub const MAX_FILE_BYTES: usize = 1024 * 1024;
pub const Error = error{
    Truncated, TooLarge, BadVersion, InvalidBlock, InvalidWidth,
    NonCanonical, IntegrityMismatch, BufferTooSmall, OverlappingBuffers,
};

fn readCount(input: []const u8, cursor: *usize) Error!usize {
    var n: u64 = 0;
    for (0..5) |i| {
        if (cursor.* >= input.len) return error.Truncated;
        const byte = input[cursor.*];
        cursor.* += 1;
        const shift: u6 = @intCast(i * 7);
        n |= @as(u64, byte & 0x7f) << shift;
        if ((byte & 0x80) == 0) {
            if (i > 0 and (byte & 0x7f) == 0) return error.NonCanonical;
            if (n > @as(u64, MAX_WORDS)) return error.TooLarge;
            return @intCast(n);
        }
    }
    return error.NonCanonical;
}
fn packedBit(input: []const u8, index: usize) u16 {
    return @as(u16, (input[index / 8] >>
        @as(u3, @intCast(7 - index % 8))) & 1);
}
fn parse(input: []const u8, output: ?[]Word) Error!usize {
    if (input.len > MAX_FILE_BYTES) return error.TooLarge;
    if (input.len < 8) return error.Truncated;
    if (!std.mem.eql(u8, input[0..3], &.{ 'S', '3', 1 }))
        return error.BadVersion;
    const end = input.len - 4;
    const crc = (@as(u32, input[end]) << 24) |
        (@as(u32, input[end + 1]) << 16) |
        (@as(u32, input[end + 2]) << 8) | @as(u32, input[end + 3]);
    if (fields.crc32(input[0..end]) != crc) return error.IntegrityMismatch;
    var pos: usize = 3;
    const target = try readCount(input[0..end], &pos);
    if (output) |out| {
        if (out.len < target) return error.BufferTooSmall;
    }
    var written: usize = 0;
    while (written < target) {
        if (pos >= end) return error.Truncated;
        const tag = input[pos];
        pos += 1;
        const count = try readCount(input[0..end], &pos);
        if (count == 0 or count > MAX_BLOCK_WORDS or count > target - written)
            return error.InvalidBlock;
        if (tag == 0) {
            const map_len = (count + 1) / 2;
            if (map_len > end - pos) return error.Truncated;
            const map = input[pos .. pos + map_len];
            pos += map_len;
            if (count % 2 != 0 and (map[map_len - 1] & 0x0f) != 0)
                return error.NonCanonical;
            var nbits: usize = 0;
            for (0..count) |i| {
                const pair = map[i / 2];
                const width: u8 = if (i % 2 == 0) pair >> 4 else pair & 0x0f;
                if (width == 0 or width > 9) return error.InvalidWidth;
                nbits += width;
            }
            const bytes = (nbits + 7) / 8;
            if (bytes > end - pos) return error.Truncated;
            const payload = input[pos .. pos + bytes];
            if (nbits % 8 != 0) {
                const unused: u3 = @intCast(8 - nbits % 8);
                const mask: u8 = (@as(u8, 1) << unused) - 1;
                if ((payload[payload.len - 1] & mask) != 0)
                    return error.NonCanonical;
            }
            var bit: usize = 0;
            for (0..count) |i| {
                const pair = map[i / 2];
                const width: u8 = if (i % 2 == 0) pair >> 4 else pair & 0x0f;
                var bits: u16 = 0;
                for (0..width) |_| {
                    bits = (bits << 1) | packedBit(payload, bit);
                    bit += 1;
                }
                if (output) |out| out[written + i] = .{ .width = width, .bits = bits };
            }
            pos += bytes;
        } else if (tag == 1) {
            if (pos >= end) return error.Truncated;
            const width = input[pos];
            pos += 1;
            if (width == 0 or width > 9) return error.InvalidWidth;
            const bytes = (@as(usize, width) + 7) / 8;
            if (bytes > end - pos) return error.Truncated;
            const payload = input[pos .. pos + bytes];
            const unused: u3 = @intCast(bytes * 8 - width);
            if (unused > 0) {
                const mask: u8 = (@as(u8, 1) << unused) - 1;
                if ((payload[payload.len - 1] & mask) != 0)
                    return error.NonCanonical;
            }
            var bits: u16 = 0;
            for (0..width) |i| bits = (bits << 1) | packedBit(payload, i);
            if (output) |out| {
                for (0..count) |i| out[written + i] = .{ .width = width, .bits = bits };
            }
            pos += bytes;
        } else return error.InvalidBlock;
        written += count;
    }
    if (pos != end) return error.NonCanonical;
    return target;
}

/// Physical preflight only: exact counts/bytes, CRC, block bounds/padding.
/// No SENS D2 admission or canonical DP re-encode.
pub fn inspect(input: []const u8) Error!usize {
    return parse(input, null);
}

/// Byte ranges must be disjoint. Encoding a word into the original frame
/// while still decoding that same frame would invalidate the CRC preflight.
fn slicesOverlap(left: []const u8, right: []const u8) bool {
    if (left.len == 0 or right.len == 0) return false;
    const a = @intFromPtr(left.ptr);
    const b = @intFromPtr(right.ptr);
    // Compare distances, never potentially overflowing pointer endpoints.
    return if (a <= b) b - a < left.len else a - b < right.len;
}

/// Preflight first; malformed or aliased input cannot mutate caller's buffer.
pub fn unpack(input: []const u8, output: []Word) Error!usize {
    // Reject both full and partial aliases before reading or writing.
    if (slicesOverlap(input, std.mem.sliceAsBytes(output)))
        return error.OverlappingBuffers;
    const count = try inspect(input);
    if (output.len < count) return error.BufferTooSmall;
    return parse(input, output);
}

fn framed(body: []const u8, buf: []u8) []const u8 {
    std.debug.assert(buf.len == body.len + 4);
    @memcpy(buf[0..body.len], body);
    const crc = fields.crc32(body);
    buf[body.len] = @intCast(crc >> 24);
    buf[body.len + 1] = @intCast((crc >> 16) & 0xff);
    buf[body.len + 2] = @intCast((crc >> 8) & 0xff);
    buf[body.len + 3] = @intCast(crc & 0xff);
    return buf;
}
test "E1 v1 exact RAW wire and leading-zero widths" {
    // Exact physical source words: '10 001 01'.
    const golden = [_]u8{ 0x53, 0x33, 0x01, 3, 0, 3, 0x23, 0x20,
        0x8a, 0x36, 0x20, 0x5d, 0x52 };
    var recovered: [3]Word = undefined;
    try std.testing.expectEqual(@as(usize, 3), try unpack(&golden, &recovered));
    try std.testing.expectEqual(@as(u8, 2), recovered[0].width);
    try std.testing.expectEqual(@as(u16, 2), recovered[0].bits);
    try std.testing.expectEqual(@as(u8, 3), recovered[1].width);
    try std.testing.expectEqual(@as(u16, 1), recovered[1].bits);
    try std.testing.expectEqual(@as(u8, 2), recovered[2].width);
    try std.testing.expectEqual(@as(u16, 1), recovered[2].bits);
    var small: [2]Word = undefined;
    try std.testing.expectError(error.BufferTooSmall, unpack(&golden, &small));
}
test "E1 unpack rejects full and partial frame/output aliases without writes" {
    const golden = [_]u8{ 0x53, 0x33, 0x01, 3, 0, 3, 0x23, 0x20,
        0x8a, 0x36, 0x20, 0x5d, 0x52 };
    // Independent output still decodes the exact same physical bytes.
    var independent: [3]Word = undefined;
    try std.testing.expectEqual(@as(usize, 3),
        try unpack(&golden, &independent));
    try std.testing.expectEqual(@as(u8, 3), independent[1].width);
    try std.testing.expectEqual(@as(u16, 1), independent[1].bits);

    // Backing storage has Word alignment, but its byte view is a valid frame.
    var backing: [16]Word = undefined;
    const bytes = std.mem.sliceAsBytes(backing[0..]);
    @memcpy(bytes[0..golden.len], &golden);
    const before = bytes[0..golden.len];

    try std.testing.expectError(error.OverlappingBuffers,
        unpack(bytes[0..golden.len], backing[0..3]));
    try std.testing.expectEqualSlices(u8, &golden, before);

    // The output starts at the LAST input byte's containing Word.
    // The pointers differ: this is a partial overlap, not an identical alias.
    const partial_index = (golden.len - 1) / @sizeOf(Word);
    const partial = backing[partial_index .. partial_index + 3];
    try std.testing.expectError(error.OverlappingBuffers,
        unpack(bytes[0..golden.len], partial));
    try std.testing.expectEqualSlices(u8, &golden, bytes[0..golden.len]);
}

test "E1 v1 exact REPEAT wire and empty frame" {
    const repeated = [_]u8{ 0x53, 0x33, 1, 3, 1, 3, 1, 0x80,
        0xe5, 0xa5, 0x56, 0xab };
    var result: [3]Word = undefined;
    try std.testing.expectEqual(@as(usize, 3), try unpack(&repeated, &result));
    for (result) |word| {
        try std.testing.expectEqual(@as(u8, 1), word.width);
        try std.testing.expectEqual(@as(u16, 1), word.bits);
    }
    try std.testing.expectEqual(@as(usize, 0),
        try inspect(&.{ 0x53, 0x33, 1, 0, 0xc7, 0xca, 0x15, 0xd8 }));
}
test "E1 reject truncation, forged CRC, nonminimal count and nonzero tail" {
    const golden = [_]u8{ 0x53, 0x33, 1, 3, 0, 3, 0x23, 0x20,
        0x8a, 0x36, 0x20, 0x5d, 0x52 };
    for (0..golden.len) |end| {
        if (end < 8) {
            try std.testing.expectError(error.Truncated, inspect(golden[0..end]));
        } else {
            try std.testing.expectError(error.IntegrityMismatch, inspect(golden[0..end]));
        }
    }
    var corrupt = golden;
    corrupt[8] ^= 1;
    try std.testing.expectError(error.IntegrityMismatch, inspect(&corrupt));
    const nonzero = [_]u8{ 'S', '3', 1, 3, 0, 3, 0x23, 0x20, 0x8b };
    var buf: [13]u8 = undefined;
    try std.testing.expectError(error.NonCanonical, inspect(framed(&nonzero, &buf)));
    const overlong = [_]u8{ 'S', '3', 1, 0x83, 0, 0, 3, 0x23, 0x20, 0x8a };
    var uleb: [14]u8 = undefined;
    try std.testing.expectError(error.NonCanonical, inspect(framed(&overlong, &uleb)));
    const short_count = [_]u8{ 'S', '3', 1, 4, 0, 3, 0x23, 0x20, 0x8a };
    var mismatch: [13]u8 = undefined;
    try std.testing.expectError(error.Truncated, inspect(framed(&short_count, &mismatch)));
    const trailer = [_]u8{ 'S', '3', 1, 3, 0, 3, 0x23, 0x20, 0x8a, 0 };
    var appended: [14]u8 = undefined;
    try std.testing.expectError(error.NonCanonical, inspect(framed(&trailer, &appended)));
    const bad_width = [_]u8{ 'S', '3', 1, 3, 0, 3, 0x03, 0x20, 0x8a };
    var widths: [13]u8 = undefined;
    try std.testing.expectError(error.InvalidWidth, inspect(framed(&bad_width, &widths)));
    const other_tag = [_]u8{ 'S', '3', 1, 3, 2, 3, 0x23, 0x20, 0x8a };
    var tags: [13]u8 = undefined;
    try std.testing.expectError(error.InvalidBlock, inspect(framed(&other_tag, &tags)));
}
test "E1 transport accepts physical D2 CLOSE but never semantic truth" {
    // A single D2 CLOSE is physically sound but NOT an executable SENS root.
    const body = [_]u8{ 'S', '3', 1, 1, 1, 1, 2, 0x40 };
    var buf: [12]u8 = undefined;
    var out: [1]Word = undefined;
    try std.testing.expectEqual(@as(usize, 1), try unpack(framed(&body, &buf), &out));
    try std.testing.expectEqual(@as(u8, 2), out[0].width);
    try std.testing.expectEqual(@as(u16, 1), out[0].bits);
}
