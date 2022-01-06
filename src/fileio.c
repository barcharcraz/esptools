#if __has_include(<unistd.h>)
#include <unistd.h>
#endif


#ifdef _WIN32
#include "win32/win32_fileio.c"
#elif defined(_POSIX_VERSION)
#include "posix/posix_fileio.c"
#endif