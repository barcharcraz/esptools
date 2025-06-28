// -*- C++ -*-
module;
#include <vector>
#include <string_view>
#include <filesystem>
#include <span>
#include <variant>
#include <map>
#include <string>
#include <QtDBus>
#include <QtCore>
export module repo;
import std;
using std::map;
using std::pair;
using std::span;
using std::string;
using std::string_view;
using std::vector;
using std::filesystem::path;
using namespace std::string_view_literals;
export enum class ObjectType {
    DirTree,
    DirMeta,
    Commit,
    File,
    TombstoneCommit,
    Commitmeta,
    PayloadLink,
    FileXattrs,
    FileXattrsLink
};

constexpr bool is_meta_object(ObjectType typ) {
    using enum ObjectType;
    switch(typ) {
    case DirMeta:
    case Commit:
    case TombstoneCommit:
    case Commitmeta:
        return true;
    default:
        return false;
    }
}

constexpr string_view ObjectType_names[] = {
    "dir-tree",
    "dir-meta",
    "commit",
    "file",
    "toumbstone-commit",
    "commit-meta",
    "payload-link"
    "file-xattrs",
    "file-xattrs-link"
};



export enum class RepoMode {
    Bare,
    BareUser,
    BareUserOnly,
    ArchiveZ2,
    BareSplitXattrs
};

export constexpr string loose_path_extension(ObjectType type, RepoMode mode) {
    string result(ObjectType_names[static_cast<size_t>(type)]);
    using enum RepoMode;
    if(mode == ArchiveZ2 && !is_meta_object(type))  {
        result.push_back('z');
    }
    return result;
}

export extern constexpr path loose_path(const QByteArray checksum, ObjectType type, RepoMode mode) {
    auto checksum_string = checksum.toHex();
    path result;
    result.append(checksum_string.cbegin(), checksum_string.cbegin()+2);
    result.append(checksum_string.cbegin()+2, checksum_string.cend());
    result.replace_extension(loose_path_extension(type, mode));
    return result;
}

export extern constexpr uint32_t canonical_mode(uint32_t m) {
    return m & (0170000 | 0755);
}

struct RelatedObject {
    string name;
    QByteArray checksum;
};

export class Commit {
public:
    map<string, QVariant> metadata;
    QByteArray parent_checksum;
    vector<RelatedObject> related_objects;
    string body;
    uint64_t timestamp;
    QByteArray root_dirtree_checksum;
    QByteArray root_dirmeta_checksum;
};

export class DirMeta {
public:
    uint32_t uid = 0;
    uint32_t gid = 0;
    uint32_t mode = 0;
    vector<pair<vector<uint8_t>, vector<uint8_t>>> xattrs;
};

export class DirTreeChecksums {
public:
    QByteArray checksum;
    QByteArray meta_checksum;
};

export class DirTree {
public:
    map<string, QByteArray> files;
    map<string, DirTreeChecksums> dirs;
};

export class FileHeader {
public:
    uint32_t uid = 0;
    uint32_t gid = 0;
    uint32_t mode = 0100644;
    uint32_t rdev = 0;
    string symlink_target;
    vector<pair<vector<uint8_t>, vector<uint8_t>>> xattrs;
};

export class MoblRepo {


public:
};
