//! Freestanding envelope mechanics: no I/O, OS, libc, heap, or SENS semantic law.
//! Widths/version/profile and EOF rule belong to SENS, not this Zig module.
const std = @import("std");
pub const Field = struct { width: u8, bits: u64 };
pub const Block = struct { offset_bytes: u32, length_bytes: u32 };
pub const Error = error{
    EmptySchema, InvalidWidth, FieldOverflow, LengthOverflow,
    LengthMismatch, NonCanonicalTail, InvalidBlockMap, IntegrityMismatch,
    OverlappingBuffers,
};
fn bytesForBits(bits: usize) usize {
    return bits / 8 + @as(usize, @intFromBool(bits % 8 != 0));
}
// Caller-owned packed bytes must not overlap the structured source/destination.
// Compare address differences, not sums, to avoid pointer-range overflow.
fn overlaps(source: []const u8, destination: []const u8) bool {
    if (source.len == 0 or destination.len == 0) return false;
    const a = @intFromPtr(source.ptr);
    const b = @intFromPtr(destination.ptr);
    return if (a <= b) b - a < source.len else a - b < destination.len;
}
pub fn measure(fields: []const Field) Error!usize {
    if (fields.len == 0) return error.EmptySchema;
    var bits: usize = 0;
    for (fields) |field| {
        if (field.width == 0 or field.width > 64) return error.InvalidWidth;
        if (field.width < 64 and field.bits >> @as(u6, @intCast(field.width)) != 0)
            return error.FieldOverflow;
        if (field.width > ~@as(usize, 0) - bits) return error.LengthOverflow;
        bits += field.width;
    }
    return bits;
}
/// Exact MSB-first binary metadata; out has no implicit extra bytes.
pub fn pack(out: []u8, fields: []const Field) Error!usize {
    const size = try measure(fields);
    if (out.len != bytesForBits(size)) return error.LengthMismatch;
    // Without this check @memset can destroy input field widths or payloads.
    if (overlaps(std.mem.sliceAsBytes(fields), out)) return error.OverlappingBuffers;
    @memset(out, 0);
    var pos: usize = 0;
    for (fields) |field| {
        var n: usize = field.width;
        while (n > 0) {
            n -= 1;
            const bit: u8 = @intCast((field.bits >> @as(u6, @intCast(n))) & 1);
            out[pos / 8] |= bit << @as(u3, @intCast(7 - pos % 8));
            pos += 1;
        }
    }
    return size;
}
/// Read only with declared field widths + exact meaningful bit length.
pub fn unpack(raw: []const u8, bit_len: usize, widths: []const u8, dst: []u64) Error!void {
    if (widths.len == 0) return error.EmptySchema;
    if (widths.len != dst.len) return error.LengthMismatch;
    var count: usize = 0;
    for (widths) |width| {
        if (width == 0 or width > 64) return error.InvalidWidth;
        if (width > ~@as(usize, 0) - count) return error.LengthOverflow;
        count += width;
    }
    if (count != bit_len or raw.len != bytesForBits(count)) return error.LengthMismatch;
    if (count % 8 != 0) {
        const unused: u3 = @intCast(8 - count % 8);
        const mask: u8 = (@as(u8, 1) << unused) - 1;
        if ((raw[raw.len - 1] & mask) != 0) return error.NonCanonicalTail;
    }
    // Decoding overlapping output words could overwrite unread source bits.
    if (overlaps(raw, std.mem.sliceAsBytes(dst)) or
        overlaps(widths, std.mem.sliceAsBytes(dst)))
        return error.OverlappingBuffers;
    var pos: usize = 0;
    for (widths, 0..) |width, i| {
        var word: u64 = 0;
        for (0..width) |_| {
            const bit: u64 = @intCast((raw[pos / 8] >> @as(u3, @intCast(7 - pos % 8))) & 1);
            word = (word << 1) | bit;
            pos += 1;
        }
        dst[i] = word;
    }
}
/// Validates explicit physical block boundaries; gaps, overlaps, trailers reject.
pub fn validateBlocks(size: usize, blocks: []const Block) Error!void {
    if (blocks.len == 0) {
        if (size == 0) return;
        return error.InvalidBlockMap;
    }
    var offset: usize = 0;
    for (blocks) |block| {
        if (block.offset_bytes != offset or block.length_bytes == 0 or
            block.length_bytes > size - offset) return error.InvalidBlockMap;
        offset += block.length_bytes;
    }
    if (offset != size) return error.InvalidBlockMap;
}
/// IEEE CRC32 check of physical bytes. Corruption detection, not authentication.
pub fn crc32(data: []const u8) u32 {
    var crc: u32 = 0xffffffff;
    for (data) |b| {
        crc ^= @as(u32, b);
        for (0..8) |_| crc = (crc >> 1) ^ (0xedb88320 & (0 -% (crc & 1)));
    }
    return ~crc;
}
pub fn verifyCrc32(data: []const u8, expected: u32) Error!void {
    if (crc32(data) != expected) return error.IntegrityMismatch;
}
test "binary fields exact width and leading zeros" {
    const fields = [_]Field{ .{ .width = 2, .bits = 1 }, .{ .width = 3, .bits = 0 }, .{ .width = 3, .bits = 7 } };
    var bytes: [1]u8 = undefined;
    try std.testing.expectEqual(@as(usize, 8), try pack(&bytes, &fields));
    try std.testing.expectEqual(@as(u8, 0x47), bytes[0]);
    var decoded: [3]u64 = undefined;
    try unpack(&bytes, 8, &.{ 2, 3, 3 }, &decoded);
    try std.testing.expectEqualSlices(u64, &.{ 1, 0, 7 }, &decoded);
}
test "caller-owned field pack and unpack reject aliased bytes before writes" {
    var source = [_]Field{
        .{ .width = 2, .bits = 1 },
        .{ .width = 3, .bits = 0 },
        .{ .width = 3, .bits = 7 },
    };
    const original = source;
    const source_bytes = std.mem.sliceAsBytes(source[0..]);
    try std.testing.expectError(error.OverlappingBuffers,
        pack(source_bytes[0..1], source[0..]));
    for (source, original) |actual, expected| {
        try std.testing.expectEqual(expected.width, actual.width);
        try std.testing.expectEqual(expected.bits, actual.bits);
    }

    var destination: [3]u64 = .{ 0xaaaa, 0xbbbb, 0xcccc };
    const shared = std.mem.sliceAsBytes(destination[0..]);
    shared[0] = 0x47;
    const before = destination;
    try std.testing.expectError(error.OverlappingBuffers,
        unpack(shared[0..1], 8, &.{ 2, 3, 3 }, destination[0..]));
    try std.testing.expectEqualSlices(u64, &before, &destination);

    // A one-byte shifted source overlaps a 24-byte output without equal pointers.
    shared[1] = 0x47;
    const shifted_before = destination;
    try std.testing.expectError(error.OverlappingBuffers,
        unpack(shared[1..2], 8, &.{ 2, 3, 3 }, destination[0..]));
    try std.testing.expectEqualSlices(u64, &shifted_before, &destination);

    // Declared widths are source metadata, not mutable scratch storage.
    shared[0] = 2;
    shared[1] = 3;
    shared[2] = 3;
    const widths_before = destination;
    try std.testing.expectError(error.OverlappingBuffers,
        unpack(&.{0x47}, 8, shared[0..3], destination[0..]));
    try std.testing.expectEqualSlices(u64, &widths_before, &destination);
}

test "truncation, append and one padding bit mutant" {
    const fields = [_]Field{.{ .width = 3, .bits = 1 }};
    var bytes: [1]u8 = undefined;
    try std.testing.expectEqual(@as(usize, 3), try pack(&bytes, &fields));
    try std.testing.expectEqual(@as(u8, 0x20), bytes[0]);
    var dst: [1]u64 = undefined;
    try std.testing.expectError(error.NonCanonicalTail, unpack(&.{0x21}, 3, &.{3}, &dst));
    try std.testing.expectError(error.LengthMismatch, unpack(&.{}, 3, &.{3}, &dst));
    try std.testing.expectError(error.LengthMismatch, unpack(&.{ 0x20, 0 }, 3, &.{3}, &dst));
}
test "field overflow and block corruption reject" {
    try std.testing.expectError(error.FieldOverflow, measure(&.{.{ .width = 1, .bits = 2 }}));
    try std.testing.expectError(error.InvalidWidth, measure(&.{.{ .width = 0, .bits = 0 }}));
    try validateBlocks(5, &.{
        .{ .offset_bytes = 0, .length_bytes = 2 },
        .{ .offset_bytes = 2, .length_bytes = 3 },
    });
    try std.testing.expectError(error.InvalidBlockMap, validateBlocks(5, &.{
        .{ .offset_bytes = 0, .length_bytes = 2 },
        .{ .offset_bytes = 3, .length_bytes = 2 },
    }));
    try std.testing.expectError(error.InvalidBlockMap, validateBlocks(5, &.{
        .{ .offset_bytes = 0, .length_bytes = 2 },
        .{ .offset_bytes = 1, .length_bytes = 4 },
    }));
}
test "IEEE CRC32 golden and one-byte mutant" {
    const checksum = crc32("123456789");
    try std.testing.expectEqual(@as(u32, 0xcbf43926), checksum);
    try verifyCrc32("123456789", checksum);
    try std.testing.expectError(error.IntegrityMismatch, verifyCrc32("123456788", checksum));
}
// This envelope stores physical MSB-first bits, not a resident/function ID.
// The calling SENS law must supply the exact widths and meaningful bit length;
// Zig must not infer a domain from the bytes or grant executable authority.
test "D1..D10 exhaustive field transport keeps exact declared rung and every bit" {
    var count: usize = 0;
    var width: u8 = 1;
    while (width <= 10) : (width += 1) {
        const limit: u16 = @as(u16, 1) << @as(u4, @intCast(width));
        const bytes_needed: usize = (@as(usize, width) + 7) / 8;
        const widths = [_]u8{width};
        const other_width = [_]u8{if (width == 10) @as(u8, 9) else width + 1};
        var bits: u16 = 0;
        while (bits < limit) : (bits += 1) {
            const fields = [_]Field{.{ .width = width, .bits = bits }};
            var physical: [2]u8 = [_]u8{0xa5} ** 2;
            const bit_len = try pack(physical[0..bytes_needed], &fields);
            try std.testing.expectEqual(@as(usize, width), bit_len);

            var restored: [1]u64 = .{std.math.maxInt(u64)};
            try unpack(physical[0..bytes_needed], bit_len, &widths, &restored);
            try std.testing.expectEqual(@as(u64, bits), restored[0]);

            // Same payload bytes plus a different declared domain is NOT a
            // valid widening/truncation when the original bit length is fixed.
            var forbidden: [1]u64 = .{0xdeadbeef};
            try std.testing.expectError(error.LengthMismatch,
                unpack(physical[0..bytes_needed], bit_len, &other_width, &forbidden));
            try std.testing.expectEqual(@as(u64, 0xdeadbeef), forbidden[0]);

            if (width % 8 != 0) {
                // Mutate just ONE unused physical padding bit, never a
                // meaningful bit. Neither alias nor silent repair is allowed.
                var mutant = physical;
                mutant[bytes_needed - 1] |= 1;
                try std.testing.expectError(error.NonCanonicalTail,
                    unpack(mutant[0..bytes_needed], bit_len, &widths, &forbidden));
                try std.testing.expectEqual(@as(u64, 0xdeadbeef), forbidden[0]);
            }
            count += 1;
        }
    }
    try std.testing.expectEqual(@as(usize, 2046), count);
}

test "exact-width field envelope refuses truncated and trailing bytes" {
    const field = [_]Field{.{ .width = 10, .bits = 0b0010101010 }};
    var physical: [2]u8 = undefined;
    try std.testing.expectEqual(@as(usize, 10), try pack(&physical, &field));
    var restored: [1]u64 = .{0xdeadbeef};
    try std.testing.expectError(error.LengthMismatch,
        unpack(physical[0..1], 10, &.{10}, &restored));
    const appended = [_]u8{ physical[0], physical[1], 0 };
    try std.testing.expectError(error.LengthMismatch,
        unpack(&appended, 10, &.{10}, &restored));
    try std.testing.expectEqual(@as(u64, 0xdeadbeef), restored[0]);
}
