"""an LZ11 compressor, greedy over a 4096-byte window with chains of where
each three bytes were last seen, and IntelligentSystems' LZ13 header around
it, so that the packed file is smaller than what it unpacks to: the game
unpacks in place, from the end of a buffer as big as the result."""


def lz11(data, chain=64):
    n = len(data)
    out = bytearray([0x11, n & 0xFF, n >> 8 & 0xFF, n >> 16 & 0xFF])
    heads = {}
    previous = [-1] * n
    at = 0

    def remember(i):
        if i + 3 <= n:
            key = data[i:i + 3]
            previous[i] = heads.get(key, -1)
            heads[key] = i

    while at < n:
        flag_at = len(out)
        out.append(0)
        flags = 0
        for bit in range(8):
            if at >= n:
                break
            best_len, best_disp = 0, 0
            if at + 3 <= n:
                cand = heads.get(data[at:at + 3], -1)
                tries = 0
                limit = min(n - at, 0x10110)
                while cand >= 0 and at - cand <= 0x1000 and tries < chain:
                    length = 3
                    while length < limit and data[cand + length] == data[at + length]:
                        length += 1
                    if length > best_len:
                        best_len, best_disp = length, at - cand
                        if length == limit:
                            break
                    cand = previous[cand]
                    tries += 1
            if best_len >= 3:
                flags |= 0x80 >> bit
                d = best_disp - 1
                if best_len <= 16:
                    out += bytes([(best_len - 1) << 4 | d >> 8, d & 0xFF])
                elif best_len <= 0x110:
                    m = best_len - 0x11
                    out += bytes([m >> 4, (m & 0xF) << 4 | d >> 8, d & 0xFF])
                else:
                    m = best_len - 0x111
                    out += bytes([0x10 | m >> 12, m >> 4 & 0xFF, (m & 0xF) << 4 | d >> 8, d & 0xFF])
                for i in range(at, at + best_len):
                    remember(i)
                at += best_len
            else:
                out.append(data[at])
                remember(at)
                at += 1
        out[flag_at] = flags
    while len(out) % 4:
        out.append(0)
    return bytes(out)


def lz13(data):
    n = len(data)
    return bytes([0x13, n & 0xFF, n >> 8 & 0xFF, n >> 16 & 0xFF]) + lz11(data)
