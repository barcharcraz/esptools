// -*- C++ -*-
#include <system_error>
module;
#include <QtDBus>
#include <QtCore>
export module repo;
import std;
using std::map;
using std::pair;
using std::span;
using std::string;
using std::string_view;
using std::tuple;
using std::vector;
using std::array;
using std::filesystem::path;
using std::integral;
using std::same_as;
using std::bit_cast;
namespace ranges = std::ranges;
namespace views = std::views;
using namespace std::string_view_literals;

// serialization into gvariant
class TestRepo;
namespace gvariant {

static size_t offset_size_for(size_t n) {
    if(n > UINT32_MAX) {
        return 8;
    }
    if(n > UINT16_MAX) {
        return 4;
    }
    if(n > UINT8_MAX) {
        return 2;
    }
    if(n > 0) {
        return 1;
    }
    return 0;

}
static size_t offset_size(span<const uint8_t> data) {
    return offset_size_for(data.size());
}

size_t read_integral(span<const uint8_t> data) {
    array<uint8_t, sizeof(size_t)> buf{};
    ranges::copy(data, buf.begin());
    if constexpr(std::endian::native == std::endian::big) {
        ranges::reverse(buf);
    }
    return std::bit_cast<size_t>(buf);
}


static size_t next_offset(span<const uint8_t> data) {
    return read_integral(data.last(offset_size(data)));
}

static ranges::view auto framing_offsets(ranges::range auto&& rng, size_t offset_size) {
    return rng | views::transform(bit_cast<array<uint8_t, sizeof(size_t)>, size_t>)
        | views::transform(views::take(offset_size))
        | views::join;
}

// quick and dirty gvariant serializer, currently just serializes to a vector in-memory
struct serializer_data {
    vector<uint8_t> data_;
    vector<size_t> meta_;
};
class array_of_fixed_serializer;
class array_of_variable_serializer;
class tuple_serializer;

class serializer {
    friend ::TestRepo;
    vector<uint8_t>& data_;
    vector<size_t> meta_;
    struct container_data {
        size_t data_start;
        size_t meta_start;
        char container;
    };
    vector<container_data> container_stack;
    size_t element_size = 0;
    void push_container(char c) {
        container_stack.push_back({
            .data_start = data_.size(),
            .meta_start = meta_.size(),
            .container = c
        });
    }
    size_t data_start() {
        if(!container_stack.empty()) {
            return container_stack.back().data_start;
        } else {
            return 0;
        }
    }
    container_data verify_container(char c) {
        if(container_stack.empty()) {
            qFatal("invalid gvariant serializer state: no container");
        }
        auto data = container_stack.back();
        container_stack.pop_back();
        if(data.container != c) {
            qFatal("invalid gvariant serializer state: wrong container type");
        }
        return data;
    }
        
    void write_framing_offsets(decltype(meta_)::iterator start) {
        data_.append_range(framing_offsets(span(start, meta_.end()), offset_size(data_)));
        meta_.erase(start, meta_.end());
    }
public:
    explicit serializer(vector<uint8_t> data) 
        : data_(data) {}
    template<class R> 
        requires ranges::input_range<R> && 
        std::convertible_to<ranges::range_reference_t<R>, uint8_t>

    void serialize_fixed(R&& value) {
        data_.append_range(value);
    }
    void serialize_string(string_view str) {
        data_.append_range(str);
        data_.push_back(0);
    }
    template<class R> 
    requires ranges::input_range<R> && 
        std::convertible_to<ranges::range_reference_t<R>, uint8_t>
    void serialize_variable(R&& value) {
        if(this->element_size != 0) {
            qFatal("expected only fixed size elements");
        }
        data_.append_range(value);
        size_t offset = data_.size() - data_start();
        meta_.push_back(offset);
    }
    void begin_fixed_array(size_t element_size) {
        this->element_size = element_size;
    }
    void end_fixed_array() {
        this->element_size = 0;
    }
    void begin_variable_array() {
        push_container('a');
    }
    void end_variable_array() {
        auto [_, meta_start, _] = verify_container('a');
        write_framing_offsets(meta_.begin() + meta_start);
    }
    void begin_tuple() {
        push_container('(');
    }
    void end_tuple() {
        auto [_, meta_start, _] = verify_container('(');
        data_.append_range(
            framing_offsets(
                span(meta_.begin() + meta_start, meta_.end()) 
                    | views::reverse, 
                offset_size(data_)));
        meta_.erase(meta_.begin() + meta_start, meta_.end());
    }
    // template<class T> requires integral<T>
    // auto& operator<<(T value) {
    //     auto arr = std::bit_cast<array<const uint8_t, sizeof(T)>>(value);
    //     serialize_primitive(arr);
    //     return *this;
    // }
    // auto& operator<<(double value) {
    //     serialize_primitive(bit_cast<array<const uint8_t, sizeof value>>(value));
    //     return *this;
    // }
};

struct serializedTuple {
    span<const uint8_t> data;
    span<const uint8_t> get_varsize_member() {
        size_t offset = next_offset(data);
        auto result = data.first(offset);
        data = data.subspan(offset, data.size() - offset - offset_size(data));
        return result;
    }
    span<const uint8_t> get_fixedsize_member(size_t size) {
        auto result = data.first(size);
        data = data.subspan(size);
        return result;
    }
};

struct serializedArray {
    struct arrayOffsets {
        size_t elm_size;
        span<const uint8_t> data;
        explicit arrayOffsets(const span<const uint8_t>& value)
            : elm_size(offset_size(value)),
              data(value.subspan(next_offset(value))) {}
        size_t size() const {
            return data.size() / elm_size;
        }
        size_t operator[](size_t pos) const {
            return read_integral(data.subspan(pos * elm_size, elm_size));
        }
        size_t at(size_t pos) const {
            if(pos >= size()) {
                throw std::out_of_range("offset out of range");
            }
            return (*this)[pos];
        }
    };

    span<const uint8_t> data;
    size_t size() const {
        // offset of the start of the offsets
        // and the end of the data
        return offsets().size() / offset_size(data);
    }
    span<const uint8_t> at(size_t pos) const {
        auto off = offsets();
        size_t end = off.at(pos);
        size_t begin = pos ? off.at(pos-1) : 0;
        return span(data.begin() + begin, data.begin() + end);
    }
    span<const uint8_t> operator[](size_t pos) const {
        auto off = offsets();
        size_t end = off[pos];
        size_t begin = pos ? off[pos-1] : 0;
        return span(data.begin() + begin, data.begin() + end);
    }
private:
    arrayOffsets offsets() const {
        return arrayOffsets(data);
    }
};

struct tupleIterator {
    using difference_type = ptrdiff_t;
    using value_type = span<const uint8_t>;
    span<const unsigned char> data;

    span<const unsigned char> operator*() const {
        size_t offset = next_offset(data);
        return data.first(offset);
    }
    tupleIterator& operator++() {
        size_t offset = next_offset(data);
        size_t size = data.size() - offset - offset_size(data);
        data = data.subspan(offset, size);
        return *this;
    }
    void operator++(int) {
        ++*this;
    }
};
static_assert(std::input_iterator<tupleIterator>);
}

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

gvariant::serializer& operator<<(gvariant::serializer& ser, const DirTreeChecksums& chk) {
    ser.serialize_variable(chk.checksum);
    ser.serialize_variable(chk.meta_checksum);
    return ser;
}

export class DirTree {
public:
    map<string, QByteArray> files;
    map<string, DirTreeChecksums> dirs;
};
gvariant::serializer& operator<<(gvariant::serializer& ser, const DirTree& tree) {
    ser.begin_variable_array();
    for(auto& [k, v] : tree.files) {
        ser.begin_tuple();
        ser.serialize_string(k);
        ser.serialize_variable(v);
        ser.end_tuple();
    }
    ser.end_variable_array();
    ser.begin_variable_array();
    for(auto& [k, v] : tree.dirs) {
        ser.begin_tuple();
        ser.serialize_string(k);
        ser << v;
        ser.end_tuple();
    }
    ser.end_variable_array();
    return ser;
}

struct DirTreeChecksumEntry {
    string name;
    QByteArray checksum;
    QByteArray meta_checksum;
};

const QDBusArgument& operator<<(QDBusArgument& argument, const DirTreeChecksumEntry& entry) {
    argument.beginStructure();
    argument << entry.name;
    argument << entry.checksum;
    argument << entry.meta_checksum;
    argument.endStructure();
    return argument;
}

const QDBusArgument& operator<<(QDBusArgument& argument, const DirTree& tree) {
    argument << vector<pair<string_view, QByteArray>>(std::from_range_t{}, tree.files);
    return argument;
}

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
    QDir repo_dir;
    QDir objects_dir;
    QDir temp_dir;
public:
    static std::expected<MoblRepo, std::error_code> create_repo(const QString& path) {
        const string_view state_dirs[] = {
            "tmp",
            "extensions",
            "state",
            "refs",
            "refs/heads",
            "refs/mirrors",
            "refs/remotes",
            "objects",
        };
        QDir repo_dir(path);
        repo_dir.mkdir("");
        for(auto& dir : state_dirs) {

        }
    }
};
