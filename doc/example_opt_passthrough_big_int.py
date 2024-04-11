import ormsgpack
ormsgpack.packb(
    2**65,
)
ormsgpack.packb(
    2**65,
    option=ormsgpack.OPT_PASSTHROUGH_BIG_INT,
    default=lambda _: {"type": "bigint", "value": str(_) }
)
ormsgpack.unpackb(_)
