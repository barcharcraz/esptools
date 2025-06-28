// -*- C++ -*-
module;
#include <windows.h>
#include <comutil.h>
#include "bindfltapi.h"

#include <memory>
#include <vector>
export module bindlib;
//import std;

using namespace std;
struct MappingInfo {
    wstring virt_root;
};

export vector<MappingInfo> getMappings(const wchar_t* path) {
    ULONG bufferSize = sizeof(BINDFLT_GET_MAPPINGS_INFO) + sizeof(BINDFLT_GET_MAPPINGS_ENTRY) * 20 + 1024;
    auto buffer = std::make_unique<unsigned char[]>(bufferSize);
    _com_util::CheckError(BfGetMappings(BINDFLT_GET_MAPPINGS_FLAG_VOLUME, nullptr, path, nullptr, &bufferSize, buffer.get()));
    vector<MappingInfo> result;
    auto info = reinterpret_cast<const BINDFLT_GET_MAPPINGS_INFO*>(buffer.get());
    for(unsigned long i = 0; i < info->MappingCount; ++i) {
        result.emplace_back(wstring(reinterpret_cast<wchar_t*>(buffer.get() + info->Entries[i].VirtRootOffset), info->Entries[i].VirtRootLength/sizeof(wchar_t)));
    }
    return result;
}
