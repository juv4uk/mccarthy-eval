//! Single caller-owned transport ABI v2; no padded host structs or heap crossing.
const std = @import("std");
const t5 = @import("t5/t5.zig");
const f3c = @import("f3c/f3c.zig");
const e1 = @import("transport/e1.zig");
const e1_writer = @import("transport/e1_writer.zig");
// ABI v2 uses caller-owned byte triplets; no padded host structs cross the boundary.
/// ABI v2: the preferred boundary uses explicit [width, payload-MSB, payload-LSB]
/// per word, NOT a C struct with host-dependent padding/alignment/endianness.
/// This triplet stream is an in-process FFI representation, NEVER .sens wire.
pub export fn sens_zig_codec_abi_version() u32 { return 2; }

pub export fn sens_zig_t5_validate(src: [*]const u8, len: usize) c_int {
    if (len == 0 or len > t5.MAX_PHYSICAL_BYTES) return -1;
    t5.validatePhysical(src[0..len]) catch return -1;
    return 0;
}
pub export fn sens_zig_t5_encode_be3(
    src: [*]const u8, byte_len: usize, dst: [*]u8, capacity: usize,
) isize {
    if (byte_len == 0 or byte_len % 3 != 0) return -1;
    const count = byte_len / 3;
    if (count > t5.MAX_PHYSICAL_BYTES * 5) return -1;
    const written = t5.encodeTripletsInto(dst[0..capacity], src[0..byte_len])
        catch |err| return if (err == error.BufferTooSmall) -2 else -1;
    return @intCast(written);
}

pub export fn sens_zig_t5_decode_be3(
    src: [*]const u8, byte_len: usize, dst: [*]u8, capacity: usize,
) isize {
    if (byte_len == 0 or byte_len > t5.MAX_PHYSICAL_BYTES) return -1;
    const written = t5.decodeTripletsInto(dst[0..capacity], src[0..byte_len])
        catch |err| return if (err == error.BufferTooSmall) -2 else -1;
    return @intCast(written);
}

pub export fn sens_zig_e1_encode_be3(
    src: [*]const u8, byte_len: usize, dst: [*]u8, capacity: usize,
) isize {
    if (byte_len == 0 or byte_len % 3 != 0) return -1;
    const count = byte_len / 3;
    if (count > e1.MAX_WORDS) return -1;
    const source_addr = @intFromPtr(src);
    const dest_addr = @intFromPtr(dst);
    if (capacity > 0 and
        ((source_addr <= dest_addr and dest_addr - source_addr < byte_len) or
            (dest_addr < source_addr and source_addr - dest_addr < capacity)))
        return -1;
    var words: [e1.MAX_WORDS]e1.Word = undefined;
    var scratch: [e1.MAX_WORDS + 1]e1_writer.Step = undefined;
    for (0..count) |i| {
        const at = i * 3;
        const width = src[at];
        const bits = (@as(u16, src[at + 1]) << 8) | @as(u16, src[at + 2]);
        if (width == 0 or width > 9) return -1;
        if (bits >= (@as(u16, 1) << @as(u4, @intCast(width)))) return -1;
        words[i] = .{ .width = width, .bits = bits };
    }
    const size = e1_writer.encode(words[0..count], dst[0..capacity], scratch[0 .. count + 1])
        catch |err| return if (err == error.BufferTooSmall) -2 else -1;
    return @intCast(size);
}

/// Decode canonical E1 physical carrier to BE3 [width, MSB, LSB] triplets.
/// -1 invalid/noncanonical/alias; -2 BE3 output too small.
pub export fn sens_zig_e1_decode_be3(
    src: [*]const u8, byte_len: usize, dst: [*]u8, capacity: usize,
) isize {
    if (byte_len == 0 or byte_len > e1.MAX_FILE_BYTES) return -1;
    const source_addr = @intFromPtr(src);
    const dest_addr = @intFromPtr(dst);
    if (capacity > 0 and
        ((source_addr <= dest_addr and dest_addr - source_addr < byte_len) or
            (dest_addr < source_addr and source_addr - dest_addr < capacity)))
        return -1;
    const max_frame: usize = 64 * 1024;
    if (byte_len > max_frame) return -1;
    var words: [e1.MAX_WORDS]e1.Word = undefined;
    var scratch: [e1.MAX_WORDS + 1]e1_writer.Step = undefined;
    var replay: [max_frame]u8 = undefined;
    const count = e1_writer.inspectCanonical(
        src[0..byte_len],
        words[0..],
        scratch[0..],
        replay[0..byte_len],
    ) catch |err| return if (err == error.BufferTooSmall) -2 else -1;
    const need = count * 3;
    if (need > capacity) return -2;
    for (0..count) |i| {
        const w = words[i];
        if (w.width == 0 or w.width > 9) return -1;
        if (w.bits >= (@as(u16, 1) << @as(u4, @intCast(w.width)))) return -1;
        const at = i * 3;
        dst[at] = w.width;
        dst[at + 1] = @intCast(w.bits >> 8);
        dst[at + 2] = @intCast(w.bits & 0xff);
    }
    return @intCast(need);
}

test "E1 ABI v2 Zig exact RAW witness, rejected alias, bad widths and capacity" {
    const input = [_]u8{ 2, 0, 2, 3, 0, 1, 2, 0, 1 };
    const expected = [_]u8{ 0x53, 0x33, 1, 3, 0, 3, 0x23, 0x20,
        0x8a, 0x36, 0x20, 0x5d, 0x52 };
    var physical: [64]u8 = .{0x5c} ** 64;
    try std.testing.expectEqual(@as(isize, expected.len),
        sens_zig_e1_encode_be3(&input, input.len, &physical, physical.len));
    try std.testing.expectEqualSlices(u8, &expected, physical[0..expected.len]);
    var too_small: [8]u8 = .{0x6d} ** 8;
    try std.testing.expectEqual(@as(isize, -2),
        sens_zig_e1_encode_be3(&input, input.len, &too_small, too_small.len));
    for (too_small) |byte| try std.testing.expectEqual(@as(u8, 0x6d), byte);
    try std.testing.expectEqual(@as(isize, -1),
        sens_zig_e1_encode_be3(&.{ 10, 0, 0 }, 3, &physical, physical.len));
    try std.testing.expectEqual(@as(isize, -1),
        sens_zig_e1_encode_be3(&.{ 2, 0, 4 }, 3, &physical, physical.len));
    try std.testing.expectEqual(@as(isize, -1),
        sens_zig_e1_encode_be3(&input, input.len - 1, &physical, physical.len));
    var shared: [64]u8 = .{0x3a} ** 64;
    @memcpy(shared[0..input.len], &input);
    const before = shared;
    try std.testing.expectEqual(@as(isize, -1),
        sens_zig_e1_encode_be3(&shared, input.len, &shared, shared.len));
    try std.testing.expectEqualSlices(u8, &before, &shared);
    try std.testing.expectEqual(@as(isize, -1),
        sens_zig_e1_encode_be3(shared[0..].ptr, input.len,
            shared[input.len - 2 ..].ptr, shared.len - input.len + 2));
    try std.testing.expectEqualSlices(u8, &before, &shared);
    var be3_out: [16]u8 = .{0x11} ** 16;
    try std.testing.expectEqual(@as(isize, input.len),
        sens_zig_e1_decode_be3(&expected, expected.len, &be3_out, be3_out.len));
    try std.testing.expectEqualSlices(u8, &input, be3_out[0..input.len]);
    try std.testing.expectEqual(@as(u8, 0x11), be3_out[input.len]);
    var tiny: [2]u8 = .{0x22} ** 2;
    try std.testing.expectEqual(@as(isize, -2),
        sens_zig_e1_decode_be3(&expected, expected.len, &tiny, tiny.len));
    for (tiny) |b| try std.testing.expectEqual(@as(u8, 0x22), b);
    try std.testing.expectEqual(@as(isize, -1),
        sens_zig_e1_decode_be3(&.{0x53}, 1, &be3_out, be3_out.len));
}

test "E1 ABI v2 REPEAT keeps D2 leading zeros and rejects CRC mutant without writes" {
    // Eight independent D2:01 words share bits but never shrink to D1:1.
    // This tests Zig's physical REPEAT framing, not D2 syntax/callability.
    const input = [_]u8{ 2, 0, 1 } ** 8;
    var encoded: [64]u8 = .{0xa5} ** 64;
    const written = sens_zig_e1_encode_be3(
        &input, input.len, &encoded, encoded.len);
    try std.testing.expectEqual(@as(isize, 12), written);
    // S3 version, eight words, REPEAT block, eight repeats, exact D2 width.
    try std.testing.expectEqualSlices(u8,
        &.{ 0x53, 0x33, 1, 8, 1, 8, 2, 0x40 }, encoded[0..8]);
    try std.testing.expectEqual(@as(u8, 0xa5), encoded[12]);

    var restored: [input.len + 1]u8 = .{0x79} ** (input.len + 1);
    try std.testing.expectEqual(@as(isize, input.len),
        sens_zig_e1_decode_be3(encoded[0..12].ptr, 12,
            &restored, restored.len));
    try std.testing.expectEqualSlices(u8, &input, restored[0..input.len]);
    try std.testing.expectEqual(@as(u8, 0x79), restored[input.len]);

    // Decoding a canonical REPEAT still needs enough output for all words.
    var short: [input.len - 1]u8 = .{0x6d} ** (input.len - 1);
    try std.testing.expectEqual(@as(isize, -2),
        sens_zig_e1_decode_be3(encoded[0..12].ptr, 12,
            &short, short.len));
    for (short) |byte| try std.testing.expectEqual(@as(u8, 0x6d), byte);

    var mutant = encoded;
    mutant[11] ^= 1; // One-bit CRC mutation, not a new SENS word.
    restored = .{0x79} ** (input.len + 1);
    try std.testing.expectEqual(@as(isize, -1),
        sens_zig_e1_decode_be3(mutant[0..12].ptr, 12,
            &restored, restored.len));
    for (restored) |byte| try std.testing.expectEqual(@as(u8, 0x79), byte);
}

test "ABI v2 Zig is the mechanical T5 ingress and egress without code registry" {
    const be3 = [_]u8{ 2, 0, 2, 3, 0, 1, 2, 0, 0, 3, 0, 0, 2, 0, 1 };
    var carrier: [16]u8 = undefined;
    try std.testing.expectEqual(@as(isize, 4),
        sens_zig_t5_encode_be3(&be3, be3.len, &carrier, carrier.len));
    try std.testing.expectEqualSlices(u8, &.{ 0x63, 0x89, 0x06, 0xa1 }, carrier[0..4]);
    var decoded: [18]u8 = .{0x75} ** 18;
    try std.testing.expectEqual(@as(isize, be3.len),
        sens_zig_t5_decode_be3(&carrier, 4, &decoded, decoded.len));
    try std.testing.expectEqualSlices(u8, &be3, decoded[0..be3.len]);
    try std.testing.expectEqual(@as(u8, 0x75), decoded[be3.len]);
    var short: [14]u8 = .{0x42} ** 14;
    try std.testing.expectEqual(@as(isize, -2),
        sens_zig_t5_decode_be3(&carrier, 4, &short, short.len));
    for (short) |b| try std.testing.expectEqual(@as(u8, 0x42), b);
    try std.testing.expectEqual(@as(isize, -1),
        sens_zig_t5_decode_be3(&.{0xf3}, 1, &decoded, decoded.len));
    try std.testing.expectEqual(@as(isize, -1),
        sens_zig_t5_decode_be3(&.{ 0, 0 }, 2, &decoded, decoded.len));
    var same: [32]u8 = .{0x77} ** 32;
    @memcpy(same[0..4], carrier[0..4]);
    try std.testing.expectEqual(@as(isize, -1),
        sens_zig_t5_decode_be3(&same, 4, &same, same.len));
    try std.testing.expectEqualSlices(u8, carrier[0..4], same[0..4]);
}

pub export fn sens_zig_f3c_pack(src: [*]const u8, len: usize, dst: [*]u8, cap: usize) isize {
    if (len == 0 or len > f3c.MAX_TINY_T5_BYTES) return -1;
    var scratch: [f3c.ABI_SCRATCH_BYTES]u8 = undefined;
    const written = (f3c.encodeInto(src[0..len], dst[0..cap], &scratch)
        catch |err| return if (err == error.BufferTooSmall) -2 else -1) orelse return -3;
    return @intCast(written);
}
pub export fn sens_zig_f3c_unpack(src: [*]const u8, len: usize, dst: [*]u8, cap: usize) isize {
    if (len < 2 or len >= f3c.MAX_TINY_T5_BYTES) return -1;
    var scratch: [f3c.ABI_SCRATCH_BYTES]u8 = undefined;
    const written = f3c.decodeInto(src[0..len], dst[0..cap], &scratch)
        catch |err| return if (err == error.BufferTooSmall) -2 else -1;
    return @intCast(written);
}
test "no-allocation-crossing Zig C ABI roundtrip and caller bounds" {
    try std.testing.expectEqual(@as(u32, 2), sens_zig_codec_abi_version());
    const words_be3 = [_]u8{ 2, 0, 2, 3, 0, 1, 2, 0, 0, 3, 0, 0, 2, 0, 1 };
    var buf: [32]u8 = undefined;
    try std.testing.expectEqual(@as(isize, -2),
        sens_zig_t5_encode_be3(&words_be3, words_be3.len, &buf, 3));
    try std.testing.expectEqual(@as(isize, 4),
        sens_zig_t5_encode_be3(&words_be3, words_be3.len, &buf, buf.len));
    try std.testing.expectEqualSlices(u8, &.{ 0x63, 0x89, 0x06, 0xa1 }, buf[0..4]);
    try std.testing.expectEqual(@as(c_int, 0), sens_zig_t5_validate(&buf, 4));
    try std.testing.expectEqual(@as(isize, 3), sens_zig_f3c_pack(&buf, 4, &buf, buf.len));
    try std.testing.expectEqualSlices(u8, &.{ 0xf3, 0x40, 0x60 }, buf[0..3]);
    try std.testing.expectEqual(@as(isize, -2), sens_zig_f3c_unpack(&buf, 3, &buf, 3));
    try std.testing.expectEqual(@as(isize, 4), sens_zig_f3c_unpack(&buf, 3, &buf, buf.len));
    try std.testing.expectEqualSlices(u8, &.{ 0x63, 0x89, 0x06, 0xa1 }, buf[0..4]);
    try std.testing.expectEqual(@as(c_int, -1), sens_zig_t5_validate(&.{0xf3}, 1));
}

test "ABI v2 big endian triplets reject Rust layout and preserve exact word widths" {
    const input = [_]u8{ 2, 0, 2, 3, 0, 1, 2, 0, 0, 3, 0, 0, 2, 0, 1 };
    var buf: [32]u8 = undefined;
    try std.testing.expectEqual(@as(isize, 4),
        sens_zig_t5_encode_be3(&input, input.len, &buf, buf.len));
    try std.testing.expectEqualSlices(u8, &.{ 0x63, 0x89, 0x06, 0xa1 }, buf[0..4]);
    try std.testing.expectEqual(@as(isize, -2),
        sens_zig_t5_encode_be3(&input, input.len, &buf, 3));
    try std.testing.expectEqual(@as(isize, -1),
        sens_zig_t5_encode_be3(&input, input.len - 1, &buf, buf.len));
    try std.testing.expectEqual(@as(isize, -1),
        sens_zig_t5_encode_be3(&.{ 0, 0, 1 }, 3, &buf, buf.len));
    try std.testing.expectEqual(@as(isize, -1),
        sens_zig_t5_encode_be3(&.{ 2, 0, 4 }, 3, &buf, buf.len));
    try std.testing.expectEqual(@as(isize, -1),
        sens_zig_t5_encode_be3(&.{ 9, 2, 0 }, 3, &buf, buf.len));
    const one = [_]u8{ 1, 0, 1 };
    const two = [_]u8{ 2, 0, 1 };
    var left: [4]u8 = undefined;
    var right: [4]u8 = undefined;
    try std.testing.expectEqual(@as(isize, 1),
        sens_zig_t5_encode_be3(&one, one.len, &left, left.len));
    try std.testing.expectEqual(@as(isize, 1),
        sens_zig_t5_encode_be3(&two, two.len, &right, right.len));
    try std.testing.expectEqual(@as(u8, 0xa1), left[0]);
    try std.testing.expectEqual(@as(u8, 0x35), right[0]);
}

test "ABI v2 T5 encode rejects whole and partial alias without writes" {
    const source_be3 = [_]u8{ 2, 0, 2, 3, 0, 1, 2, 0, 0, 3, 0, 0, 2, 0, 1 };
    var storage: [32]u8 = .{0x5a} ** 32;
    @memcpy(storage[0..source_be3.len], &source_be3);
    const original = storage;
    try std.testing.expectEqual(@as(isize, -1),
        sens_zig_t5_encode_be3(&storage, source_be3.len, &storage, storage.len));
    try std.testing.expectEqualSlices(u8, &original, &storage);
    try std.testing.expectEqual(@as(isize, -1),
        sens_zig_t5_encode_be3(&storage, source_be3.len,
            storage[13..].ptr, storage.len - 13));
    try std.testing.expectEqualSlices(u8, &original, &storage);
    var output: [4]u8 = .{0x79} ** 4;
    const unchanged = [_]u8{0x79} ** 4;
    try std.testing.expectEqual(@as(isize, -2),
        sens_zig_t5_encode_be3(&source_be3, source_be3.len, &output, 3));
    try std.testing.expectEqualSlices(u8, &unchanged, &output);
    const d10 = [_]u8{ 10, 0, 0 };
    try std.testing.expectEqual(@as(isize, -1),
        sens_zig_t5_encode_be3(&d10, d10.len, &output, output.len));
    try std.testing.expectEqualSlices(u8, &unchanged, &output);
    try std.testing.expectEqual(@as(isize, 4),
        sens_zig_t5_encode_be3(&source_be3, source_be3.len, &output, output.len));
    try std.testing.expectEqualSlices(u8, &.{ 0x63, 0x89, 0x06, 0xa1 }, &output);
}

test "ABI v2 T5 decode rejects partial overlap and accepts adjacent buffers" {
    const carrier = [_]u8{ 0x63, 0x89, 0x06, 0xa1 };
    const expected = [_]u8{ 2, 0, 2, 3, 0, 1, 2, 0, 0, 3, 0, 0, 2, 0, 1 };
    var forward: [32]u8 = .{0x6e} ** 32;
    @memcpy(forward[0..carrier.len], carrier[0..]);
    const forward_original = forward;
    try std.testing.expectEqual(@as(isize, -1),
        sens_zig_t5_decode_be3(forward[0..].ptr, carrier.len,
            forward[2..].ptr, forward.len - 2));
    try std.testing.expectEqualSlices(u8, forward_original[0..], forward[0..]);
    var reverse: [32]u8 = .{0x6e} ** 32;
    @memcpy(reverse[13 .. 13 + carrier.len], carrier[0..]);
    const reverse_original = reverse;
    try std.testing.expectEqual(@as(isize, -1),
        sens_zig_t5_decode_be3(reverse[13..].ptr, carrier.len,
            reverse[0..].ptr, reverse.len));
    try std.testing.expectEqualSlices(u8, reverse_original[0..], reverse[0..]);
    var adjacent: [32]u8 = .{0x6e} ** 32;
    @memcpy(adjacent[0..carrier.len], carrier[0..]);
    try std.testing.expectEqual(@as(isize, expected.len),
        sens_zig_t5_decode_be3(adjacent[0..].ptr, carrier.len,
            adjacent[carrier.len..].ptr, adjacent.len - carrier.len));
    try std.testing.expectEqualSlices(u8, carrier[0..], adjacent[0..carrier.len]);
    try std.testing.expectEqualSlices(u8, expected[0..],
        adjacent[carrier.len .. carrier.len + expected.len]);
    try std.testing.expectEqual(@as(u8, 0x6e), adjacent[carrier.len + expected.len]);
}

test "ABI v2 F3C return codes remain mechanical and never clobber" {
    // Both Rust and Rust-free C hosts call these exact Zig ABI v2 exports.
    // A transport error is NOT a SENS D1:NO or semantic rejection.
    const t5_quote = [_]u8{ 0x63, 0x89, 0x06, 0xa1 };
    const f3_quote = [_]u8{ 0xf3, 0x40, 0x60 };
    const untouched = [_]u8{0x75} ** 32;
    var output: [32]u8 = untouched;

    // Refuse publication on insufficient capacity, malformed input, or
    // a valid ordinary T5 carrier that does not produce an F3C saving.
    try std.testing.expectEqual(@as(isize, -2),
        sens_zig_f3c_pack(&t5_quote, t5_quote.len, &output, 2));
    try std.testing.expectEqualSlices(u8, &untouched, &output);
    try std.testing.expectEqual(@as(isize, -1),
        sens_zig_f3c_pack(&.{0xff}, 1, &output, output.len));
    try std.testing.expectEqualSlices(u8, &untouched, &output);
    try std.testing.expectEqual(@as(isize, -3),
        sens_zig_f3c_pack(&.{0x64}, 1, &output, output.len));
    try std.testing.expectEqualSlices(u8, &untouched, &output);

    try std.testing.expectEqual(@as(isize, 3),
        sens_zig_f3c_pack(&t5_quote, t5_quote.len, &output, output.len));
    try std.testing.expectEqualSlices(u8, &f3_quote, output[0..f3_quote.len]);
    try std.testing.expectEqual(@as(u8, 0x75), output[f3_quote.len]);

    // Ingress is also fail-closed: insufficient output or a corrupt marker
    // must not mutate caller-owned bytes, including a trailing sentinel.
    output = untouched;
    try std.testing.expectEqual(@as(isize, -2),
        sens_zig_f3c_unpack(&f3_quote, f3_quote.len, &output, 3));
    try std.testing.expectEqualSlices(u8, &untouched, &output);
    try std.testing.expectEqual(@as(isize, -1),
        sens_zig_f3c_unpack(&.{ 0xf4, 0x40, 0x60 }, 3, &output, output.len));
    try std.testing.expectEqualSlices(u8, &untouched, &output);
    try std.testing.expectEqual(@as(isize, 4),
        sens_zig_f3c_unpack(&f3_quote, f3_quote.len, &output, output.len));
    try std.testing.expectEqualSlices(u8, &t5_quote, output[0..t5_quote.len]);
    try std.testing.expectEqual(@as(u8, 0x75), output[t5_quote.len]);

    // Legitimate in-place operations remain supported: scratch is internal to
    // Zig; no Rust/C allocator, struct layout, or SENS truth value crosses.
    var shared: [32]u8 = untouched;
    @memcpy(shared[0..t5_quote.len], &t5_quote);
    try std.testing.expectEqual(@as(isize, 3),
        sens_zig_f3c_pack(shared[0..].ptr, t5_quote.len,
            shared[0..].ptr, shared.len));
    try std.testing.expectEqualSlices(u8, &f3_quote, shared[0..f3_quote.len]);
    try std.testing.expectEqual(@as(isize, 4),
        sens_zig_f3c_unpack(shared[0..].ptr, f3_quote.len,
            shared[0..].ptr, shared.len));
    try std.testing.expectEqualSlices(u8, &t5_quote, shared[0..t5_quote.len]);
}
