//! Frame3 Carrier (F3C) for exact-width SENS T5, Zig 0.16.0.
//! Independent bounded rank/unrank, with exact physical-byte witnesses.
//! No external implementation provides language semantics.
//! F3C is explicit .senc 0xf3; never sniff or silently fall back to T5.
//! This module checks D3-profile program *shape*, not semantic call authority.
const std = @import("std");
const t5 = @import("../t5/t5.zig");
const Allocator = std.mem.Allocator;
const Word = t5.Word;

pub const Error = error{
    WrongCarrier,
    InvalidFrame,
    InvalidProgram,
    NotSmallerThanT5,
    FrameRange,
};
pub const MAX_TINY_T5_BYTES: usize = 16;
pub const FRAME_LABEL: u8 = 0xf3;
const FRAME_MAX_TRITS: usize = 128;
const Uint = u256;

fn same(word: Word, width: u8, bits: u16) bool {
    return word.width == width and word.bits == bits;
}

fn allowed(n: usize, pos: usize) []const u8 {
    if (pos == 0 or pos == n - 1) return &.{1};
    if (pos == 1 or pos == n - 2) return &.{0};
    return &.{ 0, 1, 2 };
}

/// Exact physical suffix counts (middle trits 0,1,2; no adjacent 2).
/// Fixed frame edges are 1,0 and 0,1. A[0]=1, A[1]=3 and
/// A[k]=2*(A[k-1]+A[k-2]); previous=2 instead gives 2*A[k-1].
/// This immutable 125 x u256 table replaces a 129 x 4 x u256 local array.
/// This is byte-frame enumeration, NOT SENS semantic authority.
const SUFFIX_COUNTS: [FRAME_MAX_TRITS - 3]Uint = blk: {
    var counts: [FRAME_MAX_TRITS - 3]Uint = undefined;
    counts[0] = 1;
    counts[1] = 3;
    for (2..counts.len) |i| {
        counts[i] = 2 * (counts[i - 1] + counts[i - 2]);
    }
    break :blk counts;
};

/// Source-level immutable data footprint, not an ELF measurement.
pub const SUFFIX_ROM_BYTES: usize = @sizeOf(@TypeOf(SUFFIX_COUNTS));

fn completions(n: usize, next_pos: usize, preceding_digit: u8) Uint {
    if (next_pos >= n - 2) return 1;
    const k = n - 2 - @max(next_pos, 2);
    if (next_pos < 2 or preceding_digit != 2 or k == 0)
        return SUFFIX_COUNTS[k];
    return 2 * SUFFIX_COUNTS[k - 1];
}

fn capacity(n: usize) Uint {
    if (n < 4 or n > FRAME_MAX_TRITS) return 0;
    return SUFFIX_COUNTS[n - 4];
}

const Bucket = struct {
    width: usize,
    lo: usize,
    hi: usize,
};

fn nextBucket(n_start: usize, byte_width: usize) Bucket {
    var n = n_start;
    var remaining: Uint = @as(Uint, 1) << @as(u8, @intCast(8 * byte_width));
    while (n <= FRAME_MAX_TRITS) : (n += 1) {
        const count = capacity(n);
        if (count > remaining) break;
        remaining -= count;
    }
    return .{ .width = byte_width, .lo = n_start, .hi = n - 1 };
}

fn lengthBucket(length: usize) ?Bucket {
    var start: usize = 4;
    var width: usize = 1;
    while (start <= FRAME_MAX_TRITS) : (width += 1) {
        const bucket = nextBucket(start, width);
        if (bucket.lo <= length and length <= bucket.hi) return bucket;
        if (bucket.hi < start) return null;
        start = bucket.hi + 1;
    }
    return null;
}

fn widthBucket(width_needed: usize) ?Bucket {
    var start: usize = 4;
    var width: usize = 1;
    while (start <= FRAME_MAX_TRITS) : (width += 1) {
        const bucket = nextBucket(start, width);
        if (width == width_needed) return bucket;
        if (bucket.hi < start) return null;
        start = bucket.hi + 1;
    }
    return null;
}

fn rank(trits: []const u8) Error!Uint {
    const n = trits.len;
    if (n < 4 or n > FRAME_MAX_TRITS) return Error.FrameRange;
    var result: Uint = 0;
    var previous: i8 = -1;
    for (trits, 0..) |digit, pos| {
        var found = false;
        for (allowed(n, pos)) |candidate| {
            if (previous == 2 and candidate == 2) continue;
            if (candidate == digit) {
                found = true;
                break;
            }
            if (candidate > digit) break;
            result += completions(n, pos + 1, candidate);
        }
        if (!found) return Error.InvalidFrame;
        previous = @intCast(digit);
    }
    return result;
}

fn unrank(allocator: Allocator, n: usize, index: Uint) ![]u8 {
    if (n < 4 or n > FRAME_MAX_TRITS or index >= capacity(n))
        return Error.FrameRange;
    const out = try allocator.alloc(u8, n);
    errdefer allocator.free(out);
    var remaining = index;
    var previous: i8 = -1;
    for (0..n) |pos| {
        var found = false;
        for (allowed(n, pos)) |digit| {
            if (previous == 2 and digit == 2) continue;
            const count = completions(n, pos + 1, digit);
            if (remaining < count) {
                out[pos] = digit;
                previous = @intCast(digit);
                found = true;
                break;
            }
            remaining -= count;
        }
        if (!found) return Error.FrameRange;
    }
    return out;
}

/// Single external D2 frame; exact words remain typed, never decimal IDs.
fn frameTrits(allocator: Allocator, words: []const Word) ![]u8 {
    if (words.len < 2 or !same(words[0], 2, 2) or !same(words[words.len - 1], 2, 1))
        return Error.InvalidFrame;
    var out: std.ArrayList(u8) = .empty;
    defer out.deinit(allocator);
    for (words, 0..) |word, i| {
        if (word.width == 0 or word.width > 9 or
            @as(u32, word.bits) >= (@as(u32, 1) << @as(u5, @intCast(word.width))))
            return Error.InvalidFrame;
        if (i >= 2 and i < words.len - 1) try out.append(allocator, 2);
        var shift: u8 = word.width;
        while (shift > 0) {
            shift -= 1;
            try out.append(allocator, @intCast((word.bits >> @as(u4, @intCast(shift))) & 1));
        }
    }
    if (out.items.len > FRAME_MAX_TRITS) return Error.FrameRange;
    _ = try rank(out.items);
    return out.toOwnedSlice(allocator);
}

fn fromTrits(trits: []const u8) Error!Word {
    if (trits.len == 0 or trits.len > 9) return Error.InvalidFrame;
    var bits: u16 = 0;
    for (trits) |d| {
        if (d > 1) return Error.InvalidFrame;
        bits = (bits << 1) | @as(u16, d);
    }
    return .{ .width = @intCast(trits.len), .bits = bits };
}

fn splitFrame(allocator: Allocator, trits: []const u8) ![]Word {
    if (trits.len < 4 or trits[0] != 1 or trits[1] != 0 or
        trits[trits.len - 2] != 0 or trits[trits.len - 1] != 1)
        return Error.InvalidFrame;

    var pieces: std.ArrayList([]const u8) = .empty;
    defer pieces.deinit(allocator);
    var begin: usize = 0;
    for (trits, 0..) |trit, index| {
        if (trit == 2) {
            try pieces.append(allocator, trits[begin..index]);
            begin = index + 1;
        } else if (trit > 2) {
            return Error.InvalidFrame;
        }
    }
    try pieces.append(allocator, trits[begin..]);

    var words: std.ArrayList(Word) = .empty;
    defer words.deinit(allocator);
    try words.append(allocator, .{ .width = 2, .bits = 2 }); // D2 OPEN
    if (pieces.items.len == 1) {
        const middle = pieces.items[0][2 .. trits.len - 2];
        if (middle.len > 0) try words.append(allocator, try fromTrits(middle));
    } else {
        const first = pieces.items[0];
        const last = pieces.items[pieces.items.len - 1];
        if (first.len <= 2 or last.len <= 2) return Error.InvalidFrame;
        try words.append(allocator, try fromTrits(first[2..]));
        for (pieces.items[1 .. pieces.items.len - 1]) |part| {
            try words.append(allocator, try fromTrits(part));
        }
        try words.append(allocator, try fromTrits(last[0 .. last.len - 2]));
    }
    try words.append(allocator, .{ .width = 2, .bits = 1 }); // D2 CLOSE
    return words.toOwnedSlice(allocator);
}

/// Mechanical bounded structure only: Zig must NOT classify D3
/// opcodes, arities, QUOTE operands, or callable heads. These are solely
/// SENS semantic questions, handled after physical decode through ABI v2.
/// This parser checks balanced D2 brackets, dot placement and nesting only.
const Parser = struct {
    words: []const Word,
    pos: usize = 0,

    fn expression(self: *Parser, depth: usize) error{InvalidProgram}!void {
        if (self.pos >= self.words.len or depth > 64) return error.InvalidProgram;
        const word = self.words[self.pos];
        if (same(word, 2, 2)) {
            self.pos += 1;
            try self.list(depth + 1);
            return;
        }
        if (word.width == 2) return error.InvalidProgram;
        self.pos += 1;
    }

    fn list(self: *Parser, depth: usize) error{InvalidProgram}!void {
        var elements: usize = 0;
        while (self.pos < self.words.len) {
            while (self.pos < self.words.len and same(self.words[self.pos], 2, 0)) {
                self.pos += 1;
            }
            if (self.pos >= self.words.len) return error.InvalidProgram;
            const next = self.words[self.pos];
            if (same(next, 2, 1)) {
                self.pos += 1;
                return;
            }
            if (same(next, 2, 3)) {
                if (elements == 0) return error.InvalidProgram;
                self.pos += 1;
                try self.expression(depth + 1);
                while (self.pos < self.words.len and same(self.words[self.pos], 2, 0)) {
                    self.pos += 1;
                }
                if (self.pos >= self.words.len or !same(self.words[self.pos], 2, 1))
                    return error.InvalidProgram;
                self.pos += 1;
                return;
            }
            try self.expression(depth + 1);
            elements += 1;
        }
        return error.InvalidProgram;
    }
};

/// Physical F3C shape constraint only, NEVER SENS semantic admission.
fn canonicalStructure(words: []const Word) bool {
    if (words.len < 2 or !same(words[0], 2, 2)) return false;
    var parser: Parser = .{ .words = words };
    parser.expression(0) catch return false;
    return parser.pos == words.len;
}

/// Internal F3 body; no tiny-policy decision. Returned carrier includes 0xf3.
fn encodeFrame(allocator: Allocator, words: []const Word) ![]u8 {
    const trits = try frameTrits(allocator, words);
    defer allocator.free(trits);
    const bucket = lengthBucket(trits.len) orelse return Error.FrameRange;
    var index: Uint = try rank(trits);
    var length = bucket.lo;
    while (length < trits.len) : (length += 1) index += capacity(length);
    const result = try allocator.alloc(u8, bucket.width + 1);
    result[0] = FRAME_LABEL;
    for (0..bucket.width) |i| {
        const shift: u8 = @intCast(8 * (bucket.width - 1 - i));
        result[i + 1] = @truncate(index >> shift);
    }
    return result;
}

/// Bounded physical F3C choice; null means retain original T5.
pub fn encode(allocator: Allocator, original_t5: []const u8) !?[]u8 {
    const words = try t5.decode(allocator, original_t5);
    defer allocator.free(words);
    if (original_t5.len > MAX_TINY_T5_BYTES or !canonicalStructure(words)) return null;
    const candidate = encodeFrame(allocator, words) catch |e| switch (e) {
        Error.FrameRange, Error.InvalidFrame => return null,
        else => return e,
    };
    errdefer allocator.free(candidate);
    if (candidate.len >= original_t5.len) {
        allocator.free(candidate);
        return null;
    }
    const recovered = decode(allocator, candidate) catch {
        allocator.free(candidate);
        return null;
    };
    defer allocator.free(recovered);
    if (!std.mem.eql(u8, original_t5, recovered)) {
        allocator.free(candidate);
        return null;
    }
    return candidate;
}

/// Strictly explicit F3C .senc -> byte-exact canonical T5.
/// Never treat invalid .senc as T5 and never execute from this module.
pub fn decode(allocator: Allocator, carrier: []const u8) ![]u8 {
    if (carrier.len < 2 or carrier.len >= MAX_TINY_T5_BYTES or
        carrier[0] != FRAME_LABEL)
        return Error.WrongCarrier;
    const bucket = widthBucket(carrier.len - 1) orelse return Error.InvalidFrame;
    var index: Uint = 0;
    for (carrier[1..]) |byte| index = (index << 8) | @as(Uint, byte);

    var n = bucket.lo;
    while (n <= bucket.hi) : (n += 1) {
        const count = capacity(n);
        if (index < count) break;
        index -= count;
    }
    if (n > bucket.hi) return Error.FrameRange;
    const trits = try unrank(allocator, n, index);
    defer allocator.free(trits);
    const words = try splitFrame(allocator, trits);
    defer allocator.free(words);
    if (!canonicalStructure(words)) return Error.InvalidProgram;

    // Physical canonicality demands byte-exact replay after decode.
    const recomposed = try frameTrits(allocator, words);
    defer allocator.free(recomposed);
    if (!std.mem.eql(u8, recomposed, trits)) return Error.InvalidFrame;
    const canonical_f3 = try encodeFrame(allocator, words);
    defer allocator.free(canonical_f3);
    if (!std.mem.eql(u8, canonical_f3, carrier)) return Error.InvalidFrame;

    const physical = try t5.encode(allocator, words);
    errdefer allocator.free(physical);
    if (physical.len > MAX_TINY_T5_BYTES or carrier.len >= physical.len)
        return Error.NotSmallerThanT5;
    return physical;
}

/// F3C transport operations can run against caller-owned, bounded SRAM.
/// The existing rank/unrank and physical canonicality algorithms stay intact;
/// this adapter replaces *system heap* with a finite FixedBufferAllocator.
/// It is an explicit scratch arena, not a zero-temporary-memory claim.
/// A firmware target must size/place this buffer and verify its own stack.
pub const ABI_SCRATCH_BYTES: usize = 8 * 1024;

/// Scratch storage is an independent temporary arena. It may not alias the
/// source (the allocator could overwrite it during parsing) or destination
/// (copying from scratch into itself is not a safe publication operation).
/// Source and destination may still alias each other when scratch is separate.
fn scratchOverlaps(scratch: []const u8, region: []const u8) bool {
    if (scratch.len == 0 or region.len == 0) return false;
    const scratch_addr = @intFromPtr(scratch.ptr);
    const region_addr = @intFromPtr(region.ptr);
    return if (scratch_addr <= region_addr)
        region_addr - scratch_addr < scratch.len
    else
        scratch_addr - region_addr < region.len;
}

/// Return null when F3C does not save bytes. No caller-output mutation on
/// failure, no-saving, or insufficient output capacity. In-place input/output
/// buffers are supported because the candidate is staged in scratch first.
pub fn encodeInto(original_t5: []const u8, out: []u8, scratch: []u8) !?usize {
    if (scratchOverlaps(scratch, original_t5) or scratchOverlaps(scratch, out))
        return error.ScratchOverlap;
    if (original_t5.len == 0 or original_t5.len > MAX_TINY_T5_BYTES)
        return Error.FrameRange;
    var bounded = std.heap.FixedBufferAllocator.init(scratch);
    const candidate = (try encode(bounded.allocator(), original_t5)) orelse return null;
    if (candidate.len > out.len) return error.BufferTooSmall;
    @memcpy(out[0..candidate.len], candidate);
    return candidate.len;
}

/// Decode an explicit F3C carrier into caller-owned canonical T5 bytes.
/// All parsing, canonicality checks and size checks finish before writing.
pub fn decodeInto(carrier: []const u8, out: []u8, scratch: []u8) !usize {
    if (scratchOverlaps(scratch, carrier) or scratchOverlaps(scratch, out))
        return error.ScratchOverlap;
    var bounded = std.heap.FixedBufferAllocator.init(scratch);
    const physical = try decode(bounded.allocator(), carrier);
    if (physical.len > out.len) return error.BufferTooSmall;
    @memcpy(out[0..physical.len], physical);
    return physical.len;
}

test "F3C scratch disjointness rejects host buffer alias without mutation" {
    // Host ABIs own the scratch arena. Reject both overlap orientations before
    // creating an allocator; independently staged source/output may still alias.
    var scratch: [ABI_SCRATCH_BYTES]u8 = [_]u8{0xa5} ** ABI_SCRATCH_BYTES;
    const quote_t5 = [_]u8{ 0x63, 0x89, 0x06, 0xa1 };
    const quote_f3 = [_]u8{ 0xf3, 0x40, 0x60 };
    var output: [32]u8 = [_]u8{0x75} ** 32;

    // Destination touches scratch: no candidate is allocated or copied.
    try std.testing.expectError(error.ScratchOverlap,
        encodeInto(&quote_t5, scratch[4..36], &scratch));
    try std.testing.expectError(error.ScratchOverlap,
        decodeInto(&quote_f3, scratch[4..36], &scratch));
    for (scratch[0..36]) |byte| try std.testing.expectEqual(@as(u8, 0xa5), byte);

    // Source overlaps scratch, including a nonzero offset inside the arena.
    @memcpy(scratch[11..15], &quote_t5);
    try std.testing.expectError(error.ScratchOverlap,
        encodeInto(scratch[11..15], &output, &scratch));
    @memcpy(scratch[21..24], &quote_f3);
    try std.testing.expectError(error.ScratchOverlap,
        decodeInto(scratch[21..24], &output, &scratch));
    for (output) |byte| try std.testing.expectEqual(@as(u8, 0x75), byte);
}

test "F3C bounded SRAM adapters match golden quote and nested carriers" {
    var scratch: [ABI_SCRATCH_BYTES]u8 = undefined;
    var buffer: [32]u8 = undefined;
    const quote_t5 = [_]u8{ 0x63, 0x89, 0x06, 0xa1 };
    @memcpy(buffer[0..quote_t5.len], &quote_t5);
    const quote_packed = (try encodeInto(buffer[0..quote_t5.len], buffer[0..], &scratch)).?;
    try std.testing.expectEqual(@as(usize, 3), quote_packed);
    try std.testing.expectEqualSlices(u8, &.{ 0xf3, 0x40, 0x60 }, buffer[0..quote_packed]);
    const quote_unpacked = try decodeInto(buffer[0..quote_packed], &buffer, &scratch);
    try std.testing.expectEqual(@as(usize, quote_t5.len), quote_unpacked);
    try std.testing.expectEqualSlices(u8, &quote_t5, buffer[0..quote_unpacked]);

    const nested_t5 = [_]u8{ 0x66, 0x38, 0x67, 0x89, 0x89, 0x3b, 0x35 };
    @memcpy(buffer[0..nested_t5.len], &nested_t5);
    const nested_packed = (try encodeInto(buffer[0..nested_t5.len], &buffer, &scratch)).?;
    try std.testing.expectEqualSlices(u8, &.{ 0xf3, 0x36, 0x5f, 0x1b, 0xd0, 0xc9 },
        buffer[0..nested_packed]);
    const nested_unpacked = try decodeInto(buffer[0..nested_packed], &buffer, &scratch);
    try std.testing.expectEqualSlices(u8, &nested_t5, buffer[0..nested_unpacked]);
}

test "F3C bounded SRAM rejects errors without touching caller bytes" {
    var scratch: [ABI_SCRATCH_BYTES]u8 = undefined;
    var buffer: [32]u8 = [_]u8{0xa5} ** 32;
    const quote_t5 = [_]u8{ 0x63, 0x89, 0x06, 0xa1 };
    const quote_f3 = [_]u8{ 0xf3, 0x40, 0x60 };
    try std.testing.expectError(error.BufferTooSmall,
        encodeInto(&quote_t5, buffer[0..2], &scratch));
    try std.testing.expectError(error.BufferTooSmall,
        decodeInto(&quote_f3, buffer[0..3], &scratch));
    try std.testing.expect((try encodeInto(&.{0x64}, &buffer, &scratch)) == null);
    try std.testing.expectError(Error.WrongCarrier,
        decodeInto(&.{ 0xf4, 0x40, 0x60 }, &buffer, &scratch));
    try std.testing.expectError(t5.Error.InvalidPhysicalByte,
        encodeInto(&.{0xff}, &buffer, &scratch));
    try std.testing.expectEqualSlices(u8, &([_]u8{0xa5} ** 32), &buffer);
}

test "F3C mechanical shape must not decide D3 callability" {
    // The head is a D9 physical word, not a D3 callable function.
    // Transport preserves width and bits; SENS alone can decide meaning.
    const opaque_head = [_]Word{
        .{ .width = 2, .bits = 2 },
        .{ .width = 9, .bits = 0 },
        .{ .width = 2, .bits = 0 },
        .{ .width = 3, .bits = 1 },
        .{ .width = 2, .bits = 1 },
    };
    try std.testing.expect(canonicalStructure(&opaque_head));
    const carrier = try t5.encode(std.testing.allocator, &opaque_head);
    defer std.testing.allocator.free(carrier);
    const restored = try t5.decode(std.testing.allocator, carrier);
    defer std.testing.allocator.free(restored);
    try std.testing.expectEqualSlices(Word, &opaque_head, restored);

    const bad_dot = [_]Word{
        .{ .width = 2, .bits = 2 },
        .{ .width = 2, .bits = 3 },
        .{ .width = 2, .bits = 1 },
    };
    try std.testing.expect(!canonicalStructure(&bad_dot));
    const no_close = [_]Word{
        .{ .width = 2, .bits = 2 },
        .{ .width = 9, .bits = 0 },
    };
    try std.testing.expect(!canonicalStructure(&no_close));
}

test "F3C physical golden: QUOTE data exact bytes" {
    const a = std.testing.allocator;
    const original = [_]u8{ 0x63, 0x89, 0x06, 0xa1 };
    const encoded = (try encode(a, &original)).?;
    defer a.free(encoded);
    try std.testing.expectEqualSlices(u8, &.{ 0xf3, 0x40, 0x60 }, encoded);
    const recovered = try decode(a, &.{ 0xf3, 0x40, 0x60 });
    defer a.free(recovered);
    try std.testing.expectEqualSlices(u8, &original, recovered);
}

test "F3C refuses non-saving empty shell and noncanonical physical inputs" {
    const a = std.testing.allocator;
    try std.testing.expect((try encode(a, &.{0x64})) == null);
    try std.testing.expectError(Error.NotSmallerThanT5, decode(a, &.{ 0xf3, 0x00 }));
    try std.testing.expectError(Error.WrongCarrier, decode(a, &.{}));
    try std.testing.expectError(Error.WrongCarrier, decode(a, &.{0xf3}));
    try std.testing.expectError(Error.WrongCarrier, decode(a, &.{ 0xf4, 0x40, 0x60 }));
    const too_long = [_]u8{ 0xf3, 0xf3, 0xf3, 0xf3, 0xf3, 0xf3, 0xf3, 0xf3,
        0xf3, 0xf3, 0xf3, 0xf3, 0xf3, 0xf3, 0xf3, 0xf3 };
    try std.testing.expectError(Error.WrongCarrier, decode(a, &too_long));
    try std.testing.expectError(t5.Error.InvalidPhysicalByte, encode(a, &.{0xf3}));
}

test "F3C physical golden: nested CAR CONS .senc roundtrip" {
    const a = std.testing.allocator;
    const source = [_]u8{ 0x66, 0x38, 0x67, 0x89, 0x89, 0x3b, 0x35 };
    const encoded = (try encode(a, &source)).?;
    defer a.free(encoded);
    try std.testing.expectEqualSlices(u8, &.{ 0xf3, 0x36, 0x5f, 0x1b, 0xd0, 0xc9 }, encoded);
    const recovered = try decode(a, encoded);
    defer a.free(recovered);
    try std.testing.expectEqualSlices(u8, &source, recovered);
}


// Test-only four-state recurrence, deliberately independent of
// SUFFIX_COUNTS and completions(). It is NOT linked into production F3C.
test "F3C differential all physical suffix transitions versus four-state DP" {
    var compared: usize = 0;
    for (4..FRAME_MAX_TRITS + 1) |n| {
        // This 16,512-byte array exists only in this host test, not in
        // rank(), unrank(), capacity(), or the freestanding ABI.
        var table = std.mem.zeroes([FRAME_MAX_TRITS + 1][4]Uint);
        for (0..4) |previous| table[n][previous] = 1;
        var pos = n;
        while (pos > 0) {
            pos -= 1;
            for (0..4) |previous| {
                var count: Uint = 0;
                for (allowed(n, pos)) |digit| {
                    if (previous == 3 and digit == 2) continue;
                    count += table[pos + 1][@as(usize, digit) + 1];
                }
                table[pos][previous] = count;
            }
        }
        try std.testing.expectEqual(table[0][0], capacity(n));
        for (0..n) |position| {
            for (allowed(n, position)) |digit| {
                try std.testing.expectEqual(
                    table[position + 1][@as(usize, digit) + 1],
                    completions(n, position + 1, digit),
                );
                compared += 1;
            }
        }
    }
    try std.testing.expectEqual(@as(usize, 23750), compared);
}

test "F3C immutable suffix counts enumerate all short physical frames" {
    try std.testing.expectEqual(@as(usize, 4000), SUFFIX_ROM_BYTES);
    const a = std.testing.allocator;
    for (4..11) |n| {
        var ordinal: Uint = 0;
        while (ordinal < capacity(n)) : (ordinal += 1) {
            const digits = try unrank(a, n, ordinal);
            defer a.free(digits);
            try std.testing.expectEqual(ordinal, try rank(digits));
        }
    }
}

test "Frame3 bucket boundaries and rank/unrank without changing bit widths" {
    const a = std.testing.allocator;
    const first = widthBucket(1).?;
    try std.testing.expectEqual(@as(usize, 4), first.lo);
    try std.testing.expectEqual(@as(usize, 8), first.hi);
    const next = widthBucket(2).?;
    try std.testing.expectEqual(@as(usize, 9), next.lo);
    try std.testing.expectEqual(@as(usize, 14), next.hi);
    for ([_]usize{ 5, 7, 9, 18, 33, 64, 128 }) |n| {
        const maximum = capacity(n);
        for ([_]Uint{ 0, maximum - 1 }) |ordinal| {
            const trits = try unrank(a, n, ordinal);
            defer a.free(trits);
            try std.testing.expectEqual(ordinal, try rank(trits));
        }
    }
}
