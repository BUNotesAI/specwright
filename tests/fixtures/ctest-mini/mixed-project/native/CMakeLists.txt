cmake_minimum_required(VERSION 3.17)
project(specwright_ctest_mixed C)

enable_testing()
add_executable(ctest_route_pass route_pass.c)
add_test(NAME ctest_route_pass COMMAND ctest_route_pass)
