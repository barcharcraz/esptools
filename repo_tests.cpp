
#include <QtCore>
#include <QTest>
import repo;



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
};

#include "repo_tests.moc"

QTEST_MAIN(TestRepo)
