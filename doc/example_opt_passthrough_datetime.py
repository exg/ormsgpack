import ormsgpack, datetime
def default(obj):
    if isinstance(obj, datetime.datetime):
        return obj.strftime("%a, %d %b %Y %H:%M:%S GMT")
    raise TypeError

ormsgpack.packb({"created_at": datetime.datetime(1970, 1, 1)})
ormsgpack.packb({"created_at": datetime.datetime(1970, 1, 1)}, option=ormsgpack.OPT_PASSTHROUGH_DATETIME)
ormsgpack.packb(
    {"created_at": datetime.datetime(1970, 1, 1)},
    option=ormsgpack.OPT_PASSTHROUGH_DATETIME,
    default=default,
)
