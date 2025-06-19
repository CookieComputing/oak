#
# Copyright 2024 The Project Oak Authors
#
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#     http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.
#
"""Forwarder for bazel rules and macros."""

load("//bazel/private:oci_runtime_bundle.bzl", _oci_runtime_bundle = "oci_runtime_bundle")

oci_runtime_bundle = _oci_runtime_bundle

def any_platform(platform_list):
    """Helper to mark any platform from platform_list as compatible.

    Generates a `select` expression to use with `target_compatible_with`
    meaning that any of the platforms given is compatible.

    Example:
    ```
    target_compatible_with = any_platform([
        "//:x86_64-linux-setting",
        "//:x86_64-none-setting"
    ]),
    ```
    is equivalent to:
    ```
    target_compatible_with = select({
        "//:x86_64-linux-setting": ["//:x86_64-linux-setting"],
        "//:x86_64-none-setting": ["//:x86_64-none-setting"],
        "//conditions:default": ["@platforms//:incompatible"],
    }),
    ```
    This is the idiomatic way to select one of several possible compatible
    platforms as pointed out in
    https://bazel.build/extending/platforms#expressive-constraints,
    except we return the same OS string in the values (instead of `[]`), as
    that is required for our cquery in just bazel-ci to work properly. If we
    return `[]`, that query will include false positives, as all targets that
    don't specify any value for `target_compatible_with` will default to `[]`.
    """
    select_dict = {platform: [platform] for platform in platform_list}
    select_dict["//conditions:default"] = ["@platforms//:incompatible"]
    return select(select_dict)

def _objcopy_impl(ctx):
    out = ctx.actions.declare_file(ctx.label.name)
    cc = ctx.toolchains["@bazel_tools//tools/cpp:toolchain_type"].cc
    ctx.actions.run(
        inputs = depset(direct = [ctx.file.src], transitive = [cc.all_files]),
        outputs = [out],
        executable = cc.objcopy_executable,
        arguments = ["--output-target=binary", ctx.executable.src.path, out.path],
    )
    return [DefaultInfo(files = depset([out]))]

objcopy = rule(
    implementation = _objcopy_impl,
    attrs = {
        "src": attr.label(executable = True, cfg = "target", mandatory = True, allow_single_file = True),
    },
    toolchains = ["@bazel_tools//tools/cpp:toolchain_type"],
)

# Given an ELF file, derives a new one by setting the interpreter to the given value.
#
# Example:
# ```
# cc_binary(name = "unpatched", ...)
#
# patchelf_set_interpreter(
#   name = "patched",
#   src = ":unpatched".
#   interpreter = "/path/to/my/interpreter",  # Optional, defaults to /lib64/ld-linux-x86-64.so.2 .
# )
# ```
#
# Note: you can check which interpreter is set in an ELF file with:
# ```
# readelf -a path_to_elf | grep interpreter
# ```
def patchelf_set_interpreter(name, src, interpreter = "/lib64/ld-linux-x86-64.so.2"):
    return native.genrule(
        name = "_" + name + "_genrule",
        srcs = [src],
        outs = [name],
        cmd = 'patchelf --set-interpreter %s --set-rpath "" --output $(OUTS) $(SRCS)' % interpreter,
    )
