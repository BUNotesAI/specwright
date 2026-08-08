cmake_minimum_required(VERSION 3.17)
project(specwright_markerless_consumer NONE)

enable_testing()
add_test(
  NAME markerless_host_test
  COMMAND bash "${CMAKE_CURRENT_LIST_DIR}/../../host/run_host_test.sh"
)
set_tests_properties(markerless_host_test PROPERTIES WORKING_DIRECTORY "${CMAKE_BINARY_DIR}")
