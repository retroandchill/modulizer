//
// Created by fcors on 6/13/2026.
//
module;

#ifdef _MSC_VER
#pragma warning(disable : 5244)
#endif

export module modulizer:native;

import std;
using std::size_t;
using std::strlen;

#define MODULIZER_EXPORT_MODULE
export
{
#include <modulizer.h>
}
