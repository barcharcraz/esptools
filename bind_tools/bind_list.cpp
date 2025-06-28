#include <QtCore>
#include <comutil.h>
import bindlib;


int main(int argc, char** argv) {
    QCoreApplication app(argc, argv);
    app.setApplicationName("bind_list");
    app.setApplicationVersion("0.1.0");
    QCommandLineParser parser;
    parser.addVersionOption();
    parser.addHelpOption();

    parser.addPositionalArgument("path", "virtualization root path to list");
    parser.process(app);
    if(parser.positionalArguments().size() < 1) {
        parser.showMessageAndExit(QCommandLineParser::MessageType::Error, "bind_list is required", 1);
    }
    if(parser.positionalArguments().size() > 1) {
        parser.showMessageAndExit(QCommandLineParser::MessageType::Error, "unexpected positional option", 1);
    }
    QString virtual_root = parser.positionalArguments()[0];

    auto mappings = getMappings(reinterpret_cast<const wchar_t*>(virtual_root.utf16()));
    for(const auto& m : mappings) {

    }
    return 0;
}
