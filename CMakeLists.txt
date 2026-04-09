cmake_minimum_required(VERSION 4.1)
project(coat C)

set(CMAKE_C_STANDARD 11)

include_directories(.)

add_executable(coat
        main.c)
