#pragma once
#include <stddef.h>

#ifdef _WIN32
typedef wchar_t esppathchar;
#else 
// we hope all other platforms use char
// we could explicitly check, but it's impossible to check
// for "unix" without including <unistd.h> or doing build system stuff
// build system stuff is harder to understand than not having it and
// <unistd.h> will include extraneous symbols, so let's just assume
// it should be a pretty good assumption.
typedef char esppathchar;
#endif


#ifdef _WIN32
// HANDLE is void*
typedef void* espnativefd;
#else
typedef int espnativefd;
#endif

