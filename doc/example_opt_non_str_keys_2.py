import ormsgpack, datetime
ormsgpack.packb(
    {"1970-01-01T00:00:00": True, datetime.datetime(1970, 1, 1, 0, 0, 0): False},
    option=ormsgpack.OPT_NON_STR_KEYS,
)
ormsgpack.unpackb(_)
