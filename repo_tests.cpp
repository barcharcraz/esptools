module;
#include <QtCore>
#include <QTest>
module repo;



class TestRepo : public QObject {
    Q_OBJECT
private slots:
    void loost_path_extension() {
        using enum ObjectType;
        using enum RepoMode;
        QVERIFY(loose_path_extension(DirTree, Bare) == "dir-tree");
    }
    void  loose_path() {
        using enum ObjectType;
        using enum RepoMode;
        
        auto path = ::loose_path(QByteArray::fromHex("abcdef"), DirMeta, Bare);
        QVERIFY(path == "ab/cdef.dir-meta");
    }

    void tuple_iterator() {
        using namespace gvariant;
        const uint8_t data[] = {
            0x74, 0x65, 0x01, 0x01
        };
        tupleIterator it(data);
        QVERIFY(it.data.data() == data && it.data.size() == 4);
        QVERIFY((*it).size() == 1 && (*it)[0] == 0x74);
        ++it;
        QVERIFY(it.data.data() == data+1 && it.data.size() == 2);
        QVERIFY((*it).size() == 1 && (*it)[0] == 0x65);
        ++it;
        QVERIFY(it.data.data() == data+2 && it.data.size() == 0);
        QVERIFY((*it).size() == 0);

    }

    void framing_offsets() {
        gvariant::serializer b;
        b.data.data_.resize(100);
        b.data.meta_ = {40, 39, 5};
        b.write_framing_offsets(b.data.meta_.begin());
        QVERIFY(b.data.data_[100] == 40);
        QVERIFY(b.data.data_[101] == 39);
        QVERIFY(b.data.data_[102] == 5);
    }
};


#include "repo_tests.moc"

QTEST_MAIN(TestRepo)
