cmake_minimum_required(VERSION 3.17)
project(specwright_ctest_mini C)

enable_testing()

add_executable(ctest_pass pass.c)
add_executable(ctest_fail fail.c)

add_test(NAME ctest_pass COMMAND ctest_pass)
add_test(NAME ctest_fail COMMAND ctest_fail)
add_test(NAME ctest_missing_executable COMMAND "${CMAKE_CURRENT_BINARY_DIR}/missing-test-executable")
