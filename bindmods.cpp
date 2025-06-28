#include "bindfltapi.h"
#include <QtCore>
#include <windows.h>
#include <comutil.h>

static __attribute__((constructor)) void loadBindLink() {
    HMODULE dll =
        LoadLibraryEx(L"bindfltapi.dll", nullptr, LOAD_LIBRARY_SEARCH_SYSTEM32);
    if (!dll)
        qFatal("Loading bindlink.dll failed\n");
}

struct MappingInfo {
    QString virt_root;
};
QVector<MappingInfo> getMappings(const QString& path) {
    ULONG bufferSize = sizeof(BINDFLT_GET_MAPPINGS_INFO) + sizeof(BINDFLT_GET_MAPPINGS_ENTRY) * 20 + 1024;
    auto buffer = std::make_unique<unsigned char[]>(bufferSize);
    HRESULT hr = BfGetMappings(BINDFLT_GET_MAPPINGS_FLAG_VOLUME, nullptr, (const wchar_t*)path.utf16(), nullptr, &bufferSize, buffer.get());
    _com_util::CheckError(hr);
    QVector<MappingInfo> result;
    auto info = reinterpret_cast<const BINDFLT_GET_MAPPINGS_INFO*>(buffer.get());
    for(int i = 0; i < info->MappingCount; ++i) {
        result.push_back({QString(reinterpret_cast<QChar*>(buffer.get() + info->Entries[i].VirtRootOffset), info->Entries[i].VirtRootLength / sizeof(wchar_t))});
    }
    return result;
}

class ModList {
public:
    QString mods_folder;
    QString data_folder;
    QString modlist;

public:
    void bind() {
        auto virtual_path = QDir(data_folder).absolutePath().toStdWString();
        for (const auto &entry :
             QDirListing(mods_folder, QDirListing::IteratorFlag::DirsOnly)) {
            qInfo() << "Binding: " << entry.baseName() << "\n";

            auto backing_path = entry.absoluteFilePath().toStdWString();
            HRESULT hr = BfSetupFilter(
                nullptr,
                BINDFLT_FLAG_READ_ONLY_MAPPING | BINDFLT_FLAG_MERGED_BIND_MAPPING,
                virtual_path.c_str(), backing_path.c_str(), nullptr, 0);
            _com_util::CheckError(hr);
        }
    }
};

int main(int argc, char **argv) try {
    QCoreApplication app(argc, argv);
    QCommandLineParser parser;
    parser.addHelpOption();
    parser.addVersionOption();
    parser.addPositionalArgument(
        "source",
        "Source of the mods to bind, should be a directory of mod installs");
    parser.addPositionalArgument("dest",
                                 "destination to bind to, skyrim base directory");
    parser.addOption(
        {"modlist", "modlist.txt file to filter and order source", "path"});

    parser.process(app);

    ModList list(parser.positionalArguments()[0],
                 parser.positionalArguments()[1]);
    int attached = 0;
    _com_util::CheckError(BfAttachFilter(L"C:\\", &attached));
    qDebug("Attached: %d\n", attached);
    list.bind();
    auto res = getMappings(list.data_folder);
    for(auto& r : res) {
        qDebug() << r.virt_root << "\n";
    }
}
catch (_com_error& e) {
    qFatal() << QStringView(e.ErrorMessage()) << "\n";
}
