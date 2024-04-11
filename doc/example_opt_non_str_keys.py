import ormsgpack, datetime, uuid
ormsgpack.packb(
    {uuid.UUID("7202d115-7ff3-4c81-a7c1-2a1f067b1ece"): [1, 2, 3]},
    option=ormsgpack.OPT_NON_STR_KEYS,
)
ormsgpack.unpackb(_)
ormsgpack.packb(
    {datetime.datetime(1970, 1, 1, 0, 0, 0): [1, 2, 3]},
    option=ormsgpack.OPT_NON_STR_KEYS | ormsgpack.OPT_NAIVE_UTC,
)
ormsgpack.unpackb(_)
