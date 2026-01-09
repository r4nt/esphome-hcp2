idf_component_register(SRCS "hcp_bridge.cpp"
                       INCLUDE_DIRS "."
                       EMBED_FILES "hcp2-lp.bin")

target_link_libraries(${COMPONENT_LIB} PUBLIC "${CMAKE_CURRENT_LIST_DIR}/libhcp2_hp_lib.a")
