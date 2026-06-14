//
// Created by fcors on 6/13/2026.
//
#include <catch2/catch_test_macros.hpp>
import modulizer;

using namespace modulizer;

TEST_CASE("Can create builder", "[builder]")
{
    Builder builder;
    builder.name("test");
}
