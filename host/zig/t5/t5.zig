//! Незалежний механічний T5-декодер/кодер (Zig 0.16.0).
//! SENS залишається двійковим: трит 2 — лише межа у фізичному носії.
//! Точна ширина й біти первинні; T5 є лише фізичним носієм.
//! ВАЖЛИВО: цей модуль НЕ перевіряє D2-синтаксис і НЕ є оракулом семантики.
const std = @import("std");

pub const Word = struct {
    /// D1..D9 — саме ширина слова, НЕ десятковий ідентифікатор функції.
    width: u8,
    /// Точні біти слова; провідні нулі зберігаються полем width.
    bits: u16,
};

pub const MAX_PHYSICAL_BYTES: usize = 4 * 1024 * 1024;
const TRITS_PER_BYTE: usize = 5;

pub const Error = error{
    EmptyProgram,
    UnsupportedDomainWidth,
    TransportTooLarge,
    InvalidPhysicalByte,
    InvalidTail,
    EmptyDomainWord,
    NoncanonicalEncoding,
};

fn validWord(word: Word, max_width: u8) bool {
    if (word.width == 0 or word.width > max_width) return false;
    const limit: u32 = @as(u32, 1) << @as(u5, @intCast(word.width));
    return @as(u32, word.bits) < limit;
}

fn appendDigit(
    out: []u8,
    index: *usize,
    current: *u8,
    used: *u8,
    digit: u8,
) void {
    std.debug.assert(digit <= 2);
    current.* = current.* * 3 + digit;
    used.* += 1;
    if (used.* == TRITS_PER_BYTE) {
        out[index.*] = current.*;
        index.* += 1;
        current.* = 0;
        used.* = 0;
    }
}

/// Чистий T5 для вже типізованих слів (НЕ допуск виконуваної програми).
/// Caller owns returned bytes and must free them with the same allocator.
pub fn encode(allocator: std.mem.Allocator, words: []const Word) ![]u8 {
    return encodeWidth(allocator, words, 9);
}

/// D10 is a *mechanical coordinate* only; this does NOT enable calls.
pub fn encodeCoordinates(allocator: std.mem.Allocator, words: []const Word) ![]u8 {
    return encodeWidth(allocator, words, 10);
}

fn encodeWidth(allocator: std.mem.Allocator, words: []const Word, max_width: u8) ![]u8 {
    if (words.len == 0) return Error.EmptyProgram;

    const trit_budget = MAX_PHYSICAL_BYTES * TRITS_PER_BYTE;
    var total_trits: usize = 0;
    for (words, 0..) |word, i| {
        if (!validWord(word, max_width)) return Error.UnsupportedDomainWidth;
        const cost: usize = @as(usize, word.width) + @as(usize, @intFromBool(i != 0));
        if (cost > trit_budget - total_trits) return Error.TransportTooLarge;
        total_trits += cost;
    }
    const len = (total_trits + TRITS_PER_BYTE - 1) / TRITS_PER_BYTE;
    const out = try allocator.alloc(u8, len);
    errdefer allocator.free(out);

    var index: usize = 0;
    var current: u8 = 0;
    var used: u8 = 0;
    for (words, 0..) |word, i| {
        if (i != 0) appendDigit(out, &index, &current, &used, 2);
        var shift: u8 = word.width;
        while (shift > 0) {
            shift -= 1;
            const digit: u8 = @intCast((word.bits >> @as(u4, @intCast(shift))) & 1);
            appendDigit(out, &index, &current, &used, digit);
        }
    }
    while (used != 0) {
        appendDigit(out, &index, &current, &used, 2);
    }
    std.debug.assert(index == len);
    return out;
}

/// Caller-owned T5 encode from ABI-v2 triples [width, high bits, low bits].
/// This is an *in-process* physical bridge, not a SENS wire word or semantic ID.
/// No allocator, libc, heap, or function dispatch. Exact width stays opaque.
/// Preflight the complete source and output capacity before writing any bytes.
/// Input and written output must not overlap (checked before the first write).
pub const WriteError = Error || error{
    InvalidTriplets,
    BufferTooSmall,
    OverlappingBuffers,
};

pub fn encodeTripletsInto(out: []u8, input: []const u8) WriteError!usize {
    if (input.len == 0) return Error.EmptyProgram;
    if (input.len % 3 != 0) return error.InvalidTriplets;
    const count = input.len / 3;
    const trit_budget = MAX_PHYSICAL_BYTES * TRITS_PER_BYTE;
    var total_trits: usize = 0;

    // First pass is read-only: reject invalid words even with tiny output.
    for (0..count) |i| {
        const at = i * 3;
        const word = Word{
            .width = input[at],
            .bits = (@as(u16, input[at + 1]) << 8) | @as(u16, input[at + 2]),
        };
        if (!validWord(word, 9)) return Error.UnsupportedDomainWidth;
        const cost: usize = @as(usize, word.width) + @as(usize, @intFromBool(i != 0));
        if (cost > trit_budget - total_trits) return Error.TransportTooLarge;
        total_trits += cost;
    }

    const needed = (total_trits + TRITS_PER_BYTE - 1) / TRITS_PER_BYTE;
    if (out.len < needed) return error.BufferTooSmall;
    // Compare only the bytes actually written; never allow alias corruption.
    const src_addr = @intFromPtr(input.ptr);
    const dst_addr = @intFromPtr(out.ptr);
    if ((src_addr <= dst_addr and dst_addr - src_addr < input.len) or
        (dst_addr < src_addr and src_addr - dst_addr < needed))
        return error.OverlappingBuffers;

    var index: usize = 0;
    var current: u8 = 0;
    var used: u8 = 0;
    for (0..count) |i| {
        if (i != 0) appendDigit(out, &index, &current, &used, 2);
        const at = i * 3;
        const width = input[at];
        const bits = (@as(u16, input[at + 1]) << 8) | @as(u16, input[at + 2]);
        var shift: u8 = width;
        while (shift > 0) {
            shift -= 1;
            const digit: u8 = @intCast((bits >> @as(u4, @intCast(shift))) & 1);
            appendDigit(out, &index, &current, &used, digit);
        }
    }
    while (used != 0) appendDigit(out, &index, &current, &used, 2);
    std.debug.assert(index == needed);
    return needed;
}

/// Allocation-free T5 physical validation for hosted and freestanding
/// adapters. This checks only the wire (ternary byte range, separator, tail,
/// declared width), never a function identity, domain meaning, or SENS law.
///
/// The canonical T5 encoder emits one separator trit between opaque words and
/// pads the final physical byte with 0..4 separator trits. These conditions
/// are sufficient for byte-for-byte canonical replay without allocating words.
pub fn validatePhysical(data: []const u8) Error!void {
    return validatePhysicalWidth(data, 9);
}

/// Allocation-free D1..D10 coordinate profile. This is physical integrity
/// only: a well-formed D10 word is NOT thereby executable or resident.
pub fn validateCoordinatesPhysical(data: []const u8) Error!void {
    return validatePhysicalWidth(data, 10);
}

fn validatePhysicalWidth(data: []const u8, max_width: u8) Error!void {
    if (data.len == 0) return Error.EmptyProgram;
    if (data.len > MAX_PHYSICAL_BYTES) return Error.TransportTooLarge;
    for (data) |byte| {
        if (byte >= 243) return Error.InvalidPhysicalByte;
    }

    var tail: usize = 0;
    tail_scan: {
        var i = data.len;
        while (i > 0) {
            i -= 1;
            var byte = data[i];
            for (0..TRITS_PER_BYTE) |_| {
                if (byte % 3 != 2) break :tail_scan;
                tail += 1;
                byte /= 3;
            }
        }
    }
    if (tail >= TRITS_PER_BYTE) return Error.InvalidTail;
    const useful_trits = data.len * TRITS_PER_BYTE - tail;

    var consumed: usize = 0;
    var width: u8 = 0;
    for (data) |raw| {
        if (consumed == useful_trits) break;
        var byte = raw;
        var digits: [TRITS_PER_BYTE]u8 = undefined;
        var j: usize = TRITS_PER_BYTE;
        while (j > 0) {
            j -= 1;
            digits[j] = byte % 3;
            byte /= 3;
        }
        for (digits) |digit| {
            if (consumed == useful_trits) break;
            consumed += 1;
            if (digit == 2) {
                if (width == 0) return Error.EmptyDomainWord;
                width = 0;
            } else {
                if (width >= max_width) return Error.UnsupportedDomainWidth;
                width += 1;
            }
        }
    }
    if (width == 0) return Error.EmptyDomainWord;
}

/// Caller-owned, allocation-free T5 -> ABI-v2 byte triples [width, MSB, LSB].
/// Physical D1..D9 only. This is NOT a language code registry or admission.
/// All validation, required size and aliasing are checked before first write.
pub fn decodeTripletsInto(out: []u8, input: []const u8) WriteError!usize {
    try validatePhysical(input);

    var last = input[input.len - 1];
    var tail: usize = 0;
    while (last % 3 == 2) : (tail += 1) last /= 3;
    const useful = input.len * TRITS_PER_BYTE - tail;

    // First pass counts exact words without retaining or interpreting them.
    var word_count: usize = 1;
    var consumed: usize = 0;
    for (input) |raw| {
        if (consumed == useful) break;
        var byte = raw;
        var digits: [TRITS_PER_BYTE]u8 = undefined;
        var j: usize = TRITS_PER_BYTE;
        while (j > 0) {
            j -= 1;
            digits[j] = byte % 3;
            byte /= 3;
        }
        for (digits) |digit| {
            if (consumed == useful) break;
            consumed += 1;
            if (digit == 2) word_count += 1;
        }
    }

    const needed = word_count * 3;
    if (out.len < needed) return error.BufferTooSmall;
    const src_addr = @intFromPtr(input.ptr);
    const dst_addr = @intFromPtr(out.ptr);
    if ((src_addr <= dst_addr and dst_addr - src_addr < input.len) or
        (dst_addr < src_addr and src_addr - dst_addr < needed))
        return error.OverlappingBuffers;

    // Second pass writes only after the entire carrier has passed preflight.
    consumed = 0;
    var written: usize = 0;
    var width: u8 = 0;
    var bits: u16 = 0;
    for (input) |raw| {
        if (consumed == useful) break;
        var byte = raw;
        var digits: [TRITS_PER_BYTE]u8 = undefined;
        var j: usize = TRITS_PER_BYTE;
        while (j > 0) {
            j -= 1;
            digits[j] = byte % 3;
            byte /= 3;
        }
        for (digits) |digit| {
            if (consumed == useful) break;
            consumed += 1;
            if (digit == 2) {
                out[written] = width;
                out[written + 1] = @intCast(bits >> 8);
                out[written + 2] = @intCast(bits & 0xff);
                written += 3;
                width = 0;
                bits = 0;
            } else {
                width += 1;
                bits = (bits << 1) | @as(u16, digit);
            }
        }
    }
    out[written] = width;
    out[written + 1] = @intCast(bits >> 8);
    out[written + 2] = @intCast(bits & 0xff);
    written += 3;
    std.debug.assert(written == needed);
    return needed;
}

test "caller-owned T5 decode triplets preflights width capacity and aliasing" {
    const be3 = [_]u8{ 2, 0, 2, 3, 0, 1, 2, 0, 0, 3, 0, 0, 2, 0, 1 };
    const carrier = [_]u8{ 0x63, 0x89, 0x06, 0xa1 };
    var output: [18]u8 = .{0x7e} ** 18;
    try std.testing.expectEqual(@as(usize, be3.len), try decodeTripletsInto(&output, &carrier));
    try std.testing.expectEqualSlices(u8, &be3, output[0..be3.len]);

    var short: [14]u8 = .{0x5a} ** 14;
    try std.testing.expectError(error.BufferTooSmall, decodeTripletsInto(&short, &carrier));
    for (short) |b| try std.testing.expectEqual(@as(u8, 0x5a), b);
    try std.testing.expectError(Error.InvalidPhysicalByte, decodeTripletsInto(&output, &.{0xf3}));
    try std.testing.expectError(Error.UnsupportedDomainWidth, decodeTripletsInto(&output, &.{ 0x00, 0x00 }));
    try std.testing.expectError(Error.EmptyProgram, decodeTripletsInto(&output, &.{}));

    var overlap: [32]u8 = .{0x6b} ** 32;
    @memcpy(overlap[0..carrier.len], &carrier);
    try std.testing.expectError(error.OverlappingBuffers,
        decodeTripletsInto(overlap[0..], overlap[0..carrier.len]));
    try std.testing.expectEqualSlices(u8, &carrier, overlap[0..carrier.len]);

    // Same payload does not erase domain width on ABI v2 ingress.
    var a: [3]u8 = undefined;
    var b: [3]u8 = undefined;
    try std.testing.expectEqual(@as(usize, 3), try decodeTripletsInto(&a, &.{0xa1}));
    try std.testing.expectEqual(@as(usize, 3), try decodeTripletsInto(&b, &.{0x35}));
    try std.testing.expectEqualSlices(u8, &.{ 1, 0, 1 }, &a);
    try std.testing.expectEqualSlices(u8, &.{ 2, 0, 1 }, &b);
}

/// Декодує тільки фізичний T5; для виконання далі потрібен чинний SENS reader.
/// Caller owns returned words and must free them with the same allocator.
pub fn decode(allocator: std.mem.Allocator, data: []const u8) ![]Word {
    return decodeWidth(allocator, data, 9);
}

/// Read D1–D10 coordinates as physical words; never grant execution authority.
pub fn decodeCoordinates(allocator: std.mem.Allocator, data: []const u8) ![]Word {
    return decodeWidth(allocator, data, 10);
}

fn decodeWidth(allocator: std.mem.Allocator, data: []const u8, max_width: u8) ![]Word {
    if (data.len == 0) return Error.EmptyProgram;
    if (data.len > MAX_PHYSICAL_BYTES) return Error.TransportTooLarge;
    for (data) |byte| {
        if (byte >= 243) return Error.InvalidPhysicalByte;
    }

    // Тільки 0..4 термінальних тритів 2; цілий байт padding — помилка.
    var tail: usize = 0;
    tail_scan: {
        var i = data.len;
        while (i > 0) {
            i -= 1;
            var byte = data[i];
            var j: usize = 0;
            while (j < TRITS_PER_BYTE) : (j += 1) {
                if (byte % 3 != 2) break :tail_scan;
                tail += 1;
                byte /= 3;
            }
        }
    }
    if (tail >= TRITS_PER_BYTE) return Error.InvalidTail;
    const useful_trits = data.len * TRITS_PER_BYTE - tail;

    var decoded: std.ArrayList(Word) = .empty;
    defer decoded.deinit(allocator);
    var width: u8 = 0;
    var bits: u16 = 0;
    var consumed: usize = 0;

    for (data) |raw| {
        if (consumed >= useful_trits) break;
        var byte = raw;
        var digits: [TRITS_PER_BYTE]u8 = undefined;
        var j: usize = TRITS_PER_BYTE;
        while (j > 0) {
            j -= 1;
            digits[j] = byte % 3;
            byte /= 3;
        }
        for (digits) |digit| {
            if (consumed == useful_trits) break;
            consumed += 1;
            if (digit == 2) {
                if (width == 0) return Error.EmptyDomainWord;
                try decoded.append(allocator, .{ .width = width, .bits = bits });
                width = 0;
                bits = 0;
            } else {
                if (width >= max_width) return Error.UnsupportedDomainWidth;
                width += 1;
                bits = (bits << 1) | @as(u16, digit);
            }
        }
    }
    if (width == 0) return Error.EmptyDomainWord;
    try decoded.append(allocator, .{ .width = width, .bits = bits });

    // Приймаємо тільки канонічне фізичне представлення.
    const canonical = try encodeWidth(allocator, decoded.items, max_width);
    defer allocator.free(canonical);
    if (!std.mem.eql(u8, canonical, data)) return Error.NoncanonicalEncoding;
    return decoded.toOwnedSlice(allocator);
}

test "D10 mechanical coordinate bits do not become executable D9" {
    const a = std.testing.allocator;
    var bits: u16 = 0;
    while (bits < 1024) : (bits += 1) {
        const source = [_]Word{.{ .width = 10, .bits = bits }};
        const physical = try encodeCoordinates(a, &source);
        defer a.free(physical);
        const recovered = try decodeCoordinates(a, physical);
        defer a.free(recovered);
        try std.testing.expectEqualSlices(Word, &source, recovered);
        try std.testing.expectError(Error.UnsupportedDomainWidth, decode(a, physical));
    }
    try std.testing.expectError(
        Error.UnsupportedDomainWidth,
        encodeCoordinates(a, &.{.{ .width = 10, .bits = 1024 }}),
    );
}

test "T5 відомі фізичні вектори exact-width" {
    const Fixture = struct {
        words: []const Word,
        bytes: []const u8,
    };
    const vectors = [_]Fixture{
        .{ .words = &.{.{ .width = 1, .bits = 0 }}, .bytes = &.{0x50} },
        .{ .words = &.{.{ .width = 1, .bits = 1 }}, .bytes = &.{0xa1} },
        .{ .words = &.{.{ .width = 2, .bits = 2 }}, .bytes = &.{0x6b} },
        .{ .words = &.{.{ .width = 2, .bits = 1 }}, .bytes = &.{0x35} },
        .{ .words = &.{ .{ .width = 2, .bits = 0 }, .{ .width = 1, .bits = 1 } }, .bytes = &.{0x17} },
        .{ .words = &.{ .{ .width = 2, .bits = 2 }, .{ .width = 3, .bits = 4 }, .{ .width = 2, .bits = 1 } }, .bytes = &.{ 0x66, 0x3b } },
        .{ .words = &.{ .{ .width = 1, .bits = 1 }, .{ .width = 2, .bits = 1 }, .{ .width = 3, .bits = 1 } }, .bytes = &.{ 0x8c, 0x11 } },
        .{ .words = &.{.{ .width = 9, .bits = 256 }}, .bytes = &.{ 0x51, 0x02 } },
    };
    for (vectors) |v| {
        const physical = try encode(std.testing.allocator, v.words);
        defer std.testing.allocator.free(physical);
        try std.testing.expectEqualSlices(u8, v.bytes, physical);
        const recovered = try decode(std.testing.allocator, v.bytes);
        defer std.testing.allocator.free(recovered);
        try std.testing.expectEqualSlices(Word, v.words, recovered);
    }
}

test "кожне значення D1..D9 roundtrip без обрізання провідних нулів" {
    var width: u8 = 1;
    while (width <= 9) : (width += 1) {
        const count: u16 = @as(u16, 1) << @as(u4, @intCast(width));
        var bits: u16 = 0;
        while (bits < count) : (bits += 1) {
            const source = [_]Word{.{ .width = width, .bits = bits }};
            const physical = try encode(std.testing.allocator, &source);
            defer std.testing.allocator.free(physical);
            const recovered = try decode(std.testing.allocator, physical);
            defer std.testing.allocator.free(recovered);
            try std.testing.expectEqualSlices(Word, &source, recovered);
        }
    }
}

test "межі T5: invalid physical, tail, empty, надлишкова ширина" {
    const a = std.testing.allocator;
    try std.testing.expectError(Error.EmptyProgram, encode(a, &.{}));
    try std.testing.expectError(Error.EmptyProgram, decode(a, &.{}));
    try std.testing.expectError(Error.UnsupportedDomainWidth, encode(a, &.{.{ .width = 0, .bits = 0 }}));
    try std.testing.expectError(Error.UnsupportedDomainWidth, encode(a, &.{.{ .width = 2, .bits = 4 }}));
    try std.testing.expectError(Error.UnsupportedDomainWidth, encode(a, &.{.{ .width = 10, .bits = 0 }}));
    try std.testing.expectError(Error.InvalidPhysicalByte, decode(a, &.{0xf3}));
    try std.testing.expectError(Error.InvalidPhysicalByte, decode(a, &.{0xff}));
    try std.testing.expectError(Error.InvalidTail, decode(a, &.{0xf2}));
    try std.testing.expectError(Error.EmptyDomainWord, decode(a, &.{0xa2}));
    try std.testing.expectError(Error.EmptyDomainWord, decode(a, &.{0x4d}));
    try std.testing.expectError(Error.UnsupportedDomainWidth, decode(a, &.{ 0x00, 0x00 }));
}

test "heapless physical admission agrees with allocating T5 on all one-byte inputs" {
    const allocator = std.testing.allocator;
    for (0..256) |n| {
        const physical = [_]u8{@intCast(n)};
        const fast_ok = blk: {
            validatePhysical(&physical) catch break :blk false;
            break :blk true;
        };
        const reference_ok = blk: {
            const decoded = decode(allocator, &physical) catch break :blk false;
            allocator.free(decoded);
            break :blk true;
        };
        try std.testing.expectEqual(reference_ok, fast_ok);
    }
}

test "heapless physical D10 coordinate validator preserves all 2046 exact words" {
    const allocator = std.testing.allocator;
    var total: usize = 0;
    var width: u8 = 1;
    while (width <= 10) : (width += 1) {
        const limit: u16 = @as(u16, 1) << @as(u4, @intCast(width));
        var bits: u16 = 0;
        while (bits < limit) : (bits += 1) {
            const original = [_]Word{.{ .width = width, .bits = bits }};
            const wire = try encodeCoordinates(allocator, &original);
            defer allocator.free(wire);
            try validateCoordinatesPhysical(wire);
            const decoded = try decodeCoordinates(allocator, wire);
            defer allocator.free(decoded);
            try std.testing.expectEqualSlices(Word, &original, decoded);
            // A mechanically valid D10 carrier never opens the D1..D9
            // physical path, much less grants SENS execution rights.
            if (width == 10) {
                try std.testing.expectError(Error.UnsupportedDomainWidth,
                    validatePhysical(wire));
            }
            total += 1;
        }
    }
    try std.testing.expectEqual(@as(usize, 2046), total);
}

test "heapless D10 coordinate profile keeps D9 fail closed on physical mutants" {
    // Exactly ten zero bits is a D10 physical coordinate; it is not D9.
    try validateCoordinatesPhysical(&.{ 0x00, 0x00 });
    try std.testing.expectError(Error.UnsupportedDomainWidth,
        validatePhysical(&.{ 0x00, 0x00 }));
    try std.testing.expectError(Error.UnsupportedDomainWidth,
        validateCoordinatesPhysical(&.{ 0x00, 0x00, 0x00 }));
    try std.testing.expectError(Error.InvalidPhysicalByte,
        validateCoordinatesPhysical(&.{0xf3}));
    try std.testing.expectError(Error.InvalidTail,
        validateCoordinatesPhysical(&.{0xf2}));
    try std.testing.expectError(Error.EmptyDomainWord,
        validateCoordinatesPhysical(&.{0xa2}));
    try std.testing.expectError(Error.EmptyProgram,
        validateCoordinatesPhysical(&.{}));
}

test "heapless T5 validator preserves opaque 8-bit widths and rejects damaged wire" {
    const allocator = std.testing.allocator;
    const source = [_]Word{
        .{ .width = 8, .bits = 0xa8 },
        .{ .width = 8, .bits = 0 },
        .{ .width = 1, .bits = 1 },
    };
    const physical = try encode(allocator, &source);
    defer allocator.free(physical);
    try validatePhysical(physical);
    const recovered = try decode(allocator, physical);
    defer allocator.free(recovered);
    try std.testing.expectEqualSlices(Word, &source, recovered);
    try std.testing.expectError(Error.EmptyProgram, validatePhysical(&.{}));
    try std.testing.expectError(Error.InvalidPhysicalByte, validatePhysical(&.{0xff}));
    try std.testing.expectError(Error.InvalidTail, validatePhysical(&.{0xf2}));
    try std.testing.expectError(Error.EmptyDomainWord, validatePhysical(&.{0xa2}));
    try std.testing.expectError(Error.UnsupportedDomainWidth, validatePhysical(&.{0x00, 0x00}));
}

test "heapless ABI v2 T5 encode matches allocating physical reference across widths" {
    const a = std.testing.allocator;
    var out: [8]u8 = undefined;
    var width: u8 = 1;
    while (width <= 9) : (width += 1) {
        const maximum: u16 = (@as(u16, 1) << @as(u4, @intCast(width))) - 1;
        for ([_]u16{ 0, 1, maximum }) |bits| {
            const triplets = [_]u8{
                width, @intCast(bits >> 8), @intCast(bits & 0xff),
            };
            const reference = try encode(a, &.{.{ .width = width, .bits = bits }});
            defer a.free(reference);
            const n = try encodeTripletsInto(&out, &triplets);
            try std.testing.expectEqualSlices(u8, reference, out[0..n]);
            try validatePhysical(out[0..n]);
        }
    }
    const pair = [_]u8{ 8, 0, 0xa8, 8, 0, 0, 1, 0, 1 };
    const expected = try encode(a, &.{
        .{ .width = 8, .bits = 0xa8 },
        .{ .width = 8, .bits = 0 },
        .{ .width = 1, .bits = 1 },
    });
    defer a.free(expected);
    const n = try encodeTripletsInto(&out, &pair);
    try std.testing.expectEqualSlices(u8, expected, out[0..n]);
}

test "heapless T5 encode never writes on malformed input, tiny buffer or overlap" {
    var out: [12]u8 = [_]u8{0xa5} ** 12;
    try std.testing.expectError(Error.EmptyProgram, encodeTripletsInto(&out, &.{}));
    try std.testing.expectError(error.InvalidTriplets, encodeTripletsInto(&out, &.{ 1, 0 }));
    try std.testing.expectError(Error.UnsupportedDomainWidth, encodeTripletsInto(&out, &.{ 0, 0, 0 }));
    try std.testing.expectError(Error.UnsupportedDomainWidth, encodeTripletsInto(&out, &.{ 2, 0, 4 }));
    try std.testing.expectError(Error.UnsupportedDomainWidth, encodeTripletsInto(&out, &.{ 9, 2, 0 }));
    try std.testing.expectError(error.BufferTooSmall, encodeTripletsInto(out[0..0], &.{ 1, 0, 1 }));
    try std.testing.expectEqualSlices(u8, &([_]u8{0xa5} ** 12), &out);
    var alias: [8]u8 = [_]u8{0} ** 8;
    alias[0] = 1;
    alias[1] = 0;
    alias[2] = 1;
    try std.testing.expectError(error.OverlappingBuffers, encodeTripletsInto(&alias, alias[0..3]));
    try std.testing.expectEqualSlices(u8, &.{ 1, 0, 1 }, alias[0..3]);
}


test "canonical T5 truncation may remain a different valid coordinate" {
    // Physical-only counterexample. T5 has no embedded file length or digest.
    // D9:100000000 is 51 02, but its first byte 51 independently forms
    // D5:10000. Therefore a generic "all truncated T5 must reject" gate
    // would reject a valid coordinate without external integrity evidence.
    const a = std.testing.allocator;
    const full_word = [_]Word{.{ .width = 9, .bits = 0b100000000 }};
    const physical = try encode(a, &full_word);
    defer a.free(physical);
    try std.testing.expectEqualSlices(u8, &.{ 0x51, 0x02 }, physical);

    const shorter_physical = physical[0..1];
    try validatePhysical(physical);
    try validatePhysical(shorter_physical);

    const full_decoded = try decode(a, physical);
    defer a.free(full_decoded);
    try std.testing.expectEqualSlices(Word, &full_word, full_decoded);

    const shorter_decoded = try decode(a, shorter_physical);
    defer a.free(shorter_decoded);
    try std.testing.expectEqualSlices(Word, &.{.{
        .width = 5,
        .bits = 0b10000,
    }}, shorter_decoded);

    const shorter_reencoded = try encode(a, shorter_decoded);
    defer a.free(shorter_reencoded);
    try std.testing.expectEqualSlices(u8, shorter_physical, shorter_reencoded);

    // Both are canonical; only externally pinned size/SHA can establish
    // whether the shorter valid file was an unintended truncation.
    try std.testing.expect(physical.len != shorter_physical.len);
    try std.testing.expect(full_decoded[0].width != shorter_decoded[0].width);
}

test "all 59049 representable two-byte T5 carriers agree across validators and decoders" {
    // Physical transport property test, not SENS syntax or function admission.
    const a = std.testing.allocator;
    var accepted_nine: usize = 0;
    var accepted_coordinates: usize = 0;
    var d10_only: usize = 0;

    for (0..243) |first| {
        for (0..243) |second| {
            const wire = [_]u8{ @intCast(first), @intCast(second) };
            const fast_nine = blk: {
                validatePhysical(&wire) catch break :blk false;
                break :blk true;
            };
            const fast_coordinates = blk: {
                validateCoordinatesPhysical(&wire) catch break :blk false;
                break :blk true;
            };
            var decoded_nine = false;
            if (decode(a, &wire)) |words| {
                defer a.free(words);
                decoded_nine = true;
                const canonical = try encode(a, words);
                defer a.free(canonical);
                try std.testing.expectEqualSlices(u8, &wire, canonical);
            } else |err| {
                if (err == error.OutOfMemory) return err;
            }
            var decoded_coordinates = false;
            if (decodeCoordinates(a, &wire)) |words| {
                defer a.free(words);
                decoded_coordinates = true;
                const canonical = try encodeCoordinates(a, words);
                defer a.free(canonical);
                try std.testing.expectEqualSlices(u8, &wire, canonical);
            } else |err| {
                if (err == error.OutOfMemory) return err;
            }
            try std.testing.expectEqual(decoded_nine, fast_nine);
            try std.testing.expectEqual(decoded_coordinates, fast_coordinates);
            try std.testing.expect(!decoded_nine or decoded_coordinates);
            if (decoded_nine) accepted_nine += 1;
            if (decoded_coordinates) accepted_coordinates += 1;
            if (decoded_coordinates and !decoded_nine) d10_only += 1;
        }
    }
    // Exactly 2^10 unpadded ten-bit words occupy both physical bytes.
    // None is admitted by the D1..D9-only physical reader.
    try std.testing.expectEqual(@as(usize, 1024), d10_only);
    try std.testing.expect(accepted_nine > 0);
    try std.testing.expect(accepted_coordinates > accepted_nine);
}
