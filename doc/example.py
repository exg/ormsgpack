import ormsgpack, datetime, numpy
data = {
    "type": "job",
    "created_at": datetime.datetime(1970, 1, 1),
    "status": "🆗",
    "payload": numpy.array([[1, 2], [3, 4]]),
}
ormsgpack.packb(data, option=ormsgpack.OPT_NAIVE_UTC | ormsgpack.OPT_SERIALIZE_NUMPY)
ormsgpack.unpackb(_)
