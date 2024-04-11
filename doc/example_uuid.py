import ormsgpack, uuid
ormsgpack.packb(uuid.UUID('f81d4fae-7dec-11d0-a765-00a0c91e6bf6'))
ormsgpack.unpackb(_)
ormsgpack.packb(uuid.uuid5(uuid.NAMESPACE_DNS, "python.org"))
ormsgpack.unpackb(_)
