//
// Created by fcors on 6/13/2026.
//
export module modulizer:native;

import std;
using std::size_t;
using std::strlen;

#define MODULIZER_EXPORT_MODULE
export
{
#include <modulizer.h>
}
