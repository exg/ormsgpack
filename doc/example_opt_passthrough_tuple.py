import ormsgpack
ormsgpack.packb(
    (9193, "test", 42),
)
ormsgpack.unpackb(_)
ormsgpack.packb(
    (9193, "test", 42),
    option=ormsgpack.OPT_PASSTHROUGH_TUPLE,
    default=lambda _: {"type": "tuple", "value": list(_)}
)
ormsgpack.unpackb(_)
