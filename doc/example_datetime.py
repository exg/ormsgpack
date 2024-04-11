import ormsgpack, datetime, zoneinfo
ormsgpack.packb(
    datetime.datetime(2018, 12, 1, 2, 3, 4, 9, tzinfo=zoneinfo.ZoneInfo('Australia/Adelaide'))
)
ormsgpack.unpackb(_)
ormsgpack.packb(
    datetime.datetime.fromtimestamp(4123518902).replace(tzinfo=datetime.timezone.utc)
)
ormsgpack.unpackb(_)
ormsgpack.packb(
    datetime.datetime.fromtimestamp(4123518902)
)
ormsgpack.unpackb(_)
