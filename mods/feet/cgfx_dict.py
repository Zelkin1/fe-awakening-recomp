"""CGFX's dictionaries (DICT): a patricia tree over the entries' names, in
the order of the entries, that the game searches by name. built here the way
the game's own files are, so that a dictionary with entries added still
finds every name.

a node is 16 bytes: the bit it tests, its left and right children's indices,
then self-relative pointers to its name and its data. node 0 is the root,
testing no bit, its left child the tree's top. bit n is bit n % 8 of the
name's character n // 8, characters past its end reading 0; a node's
children test lower bits, and a link to a node that tests a bit no lower
than the one it leaves ends a search there."""
import struct

ROOT_BIT = 0xFFFFFFFF


def bit(name, n):
    data = name.encode("latin-1")
    return (data[n >> 3] >> (n & 7)) & 1 if (n >> 3) < len(data) else 0


def highest_difference(a, b):
    """the highest bit where two names differ, None when they're the same."""
    for n in range(8 * max(len(a), len(b)) - 1, -1, -1):
        if bit(a, n) != bit(b, n):
            return n
    return None


def build(names):
    """the nodes for names in that order: (bit, left, right, name) each, the
    root first."""
    nodes = [[ROOT_BIT, 0, 0, ""]]
    for name in names:
        ending = search(nodes, name)
        difference = highest_difference(name, nodes[ending][3])
        if difference is None:
            raise ValueError(f"{name} twice in a dictionary")
        # down from the root to where the new node's bit goes
        previous, current, went_right = 0, nodes[0][1], False
        while nodes[current][0] < nodes[previous][0] and nodes[current][0] > difference:
            previous = current
            went_right = bool(bit(name, nodes[current][0]))
            current = nodes[current][2] if went_right else nodes[current][1]
        index = len(nodes)
        if bit(name, difference):
            nodes.append([difference, current, index, name])
        else:
            nodes.append([difference, index, current, name])
        if previous == 0:
            nodes[0][1] = index
        elif went_right:
            nodes[previous][2] = index
        else:
            nodes[previous][1] = index
    return nodes


def search(nodes, name):
    """the node a search for name ends at, as the game searches."""
    previous, current = 0, nodes[0][1]
    while nodes[current][0] < nodes[previous][0]:
        previous = current
        current = nodes[current][2] if bit(name, nodes[current][0]) else nodes[current][1]
    return current


def pack(nodes, at, name_at, data_at):
    """the DICT's bytes, placed at at: name_at and data_at give each entry's
    name and data addresses by name. 'DICT', its size, the entry count, then
    the nodes."""
    out = bytearray(b"DICT" + struct.pack("<II", 0x0C + 0x10 * len(nodes), len(nodes) - 1))
    for i, (n, left, right, name) in enumerate(nodes):
        node = at + 0x0C + 0x10 * i
        names = name_at[name] - (node + 8) if name else 0
        data = data_at[name] - (node + 12) if name else 0
        out += struct.pack("<IHHii", n, left, right, names, data)
    return bytes(out)
