// ABOUTME: Safe Rust wrapper around the Graphviz C API (cgraph + gvc).
// ABOUTME: Provides DOT rendering (SVG, plain) and DOT syntax validation.

#[allow(non_upper_case_globals)]
#[allow(non_camel_case_types)]
#[allow(non_snake_case)]
#[allow(dead_code)]
#[allow(deref_nullptr)]
#[allow(clippy::all)]
mod bindings {
    include!(concat!(env!("OUT_DIR"), "/graphviz_bindings.rs"));
}

use bindings::*;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::ptr;
use std::sync::Mutex;

use crate::{DotError, LayoutEngine};

/// Global mutex to serialize access to the Graphviz C library, which is not thread-safe.
static GRAPHVIZ_LOCK: Mutex<()> = Mutex::new(());

extern "C" {
    static gvplugin_dot_layout_LTX_library: gvplugin_library_t;
    static gvplugin_neato_layout_LTX_library: gvplugin_library_t;
    static gvplugin_core_LTX_library: gvplugin_library_t;
}

/// Map a LayoutEngine variant to its Graphviz engine name.
fn engine_name(engine: &LayoutEngine) -> &'static str {
    match engine {
        LayoutEngine::Dot => "dot",
        LayoutEngine::Neato => "neato",
        LayoutEngine::Fdp => "fdp",
        LayoutEngine::Circo => "circo",
        LayoutEngine::Twopi => "twopi",
        LayoutEngine::Sfdp => "sfdp",
    }
}

/// Render DOT source to a specified output format using the given layout engine.
fn render_to_format(dot_source: &str, engine: &LayoutEngine, format: &str) -> Result<String, DotError> {
    let c_dot = CString::new(dot_source).map_err(|e| DotError::SyntaxError {
        message: format!("DOT source contains null byte: {}", e),
        line: 0,
        column: 0,
    })?;
    let c_engine = CString::new(engine_name(engine)).unwrap();
    let c_format = CString::new(format).unwrap();

    let _lock = GRAPHVIZ_LOCK.lock().map_err(|e| DotError::RenderError {
        message: format!("failed to acquire Graphviz lock: {}", e),
    })?;

    unsafe {
        let gvc = gvContext();
        if gvc.is_null() {
            return Err(DotError::RenderError {
                message: "failed to create Graphviz context".to_string(),
            });
        }

        // Register static plugins since LTDL is disabled.
        // SAFETY: gvAddLibrary only reads from the plugin library structs, so the
        // const-to-mut cast is safe despite the C API's mutable pointer signature.
        gvAddLibrary(gvc, &gvplugin_dot_layout_LTX_library as *const _ as *mut _);
        gvAddLibrary(gvc, &gvplugin_neato_layout_LTX_library as *const _ as *mut _);
        gvAddLibrary(gvc, &gvplugin_core_LTX_library as *const _ as *mut _);

        let graph = agmemread(c_dot.as_ptr());
        if graph.is_null() {
            gvFreeContext(gvc);
            return Err(DotError::SyntaxError {
                message: "failed to parse DOT source".to_string(),
                line: 0,
                column: 0,
            });
        }

        let layout_rc = gvLayout(gvc, graph, c_engine.as_ptr());
        if layout_rc != 0 {
            agclose(graph);
            gvFreeContext(gvc);
            return Err(DotError::LayoutError {
                message: format!("gvLayout failed with code {}", layout_rc),
            });
        }

        let mut result_ptr: *mut c_char = ptr::null_mut();
        let mut result_len: ::std::os::raw::c_uint = 0;
        let render_rc = gvRenderData(
            gvc,
            graph,
            c_format.as_ptr(),
            &mut result_ptr,
            &mut result_len,
        );

        if render_rc != 0 || result_ptr.is_null() {
            gvFreeLayout(gvc, graph);
            agclose(graph);
            gvFreeContext(gvc);
            return Err(DotError::RenderError {
                message: format!("gvRenderData failed with code {}", render_rc),
            });
        }

        let output = CStr::from_ptr(result_ptr)
            .to_string_lossy()
            .into_owned();

        // result_ptr is allocated by the Graphviz C library's internal allocator
        // and must be freed with gvFreeRenderData (not libc::free or Rust's allocator).
        gvFreeRenderData(result_ptr);
        gvFreeLayout(gvc, graph);
        agclose(graph);
        gvFreeContext(gvc);

        Ok(output)
    }
}

/// Render DOT source to SVG using the specified layout engine.
pub fn render_to_svg(dot_source: &str, engine: &LayoutEngine) -> Result<String, DotError> {
    render_to_format(dot_source, engine, "svg")
}

/// Render DOT source to Graphviz plain text format using the specified layout engine.
pub fn render_to_plain(dot_source: &str, engine: &LayoutEngine) -> Result<String, DotError> {
    render_to_format(dot_source, engine, "plain")
}

/// Validate DOT syntax by attempting to parse it.
pub fn validate_syntax(dot_source: &str) -> Result<(), DotError> {
    let c_dot = CString::new(dot_source).map_err(|e| DotError::SyntaxError {
        message: format!("DOT source contains null byte: {}", e),
        line: 0,
        column: 0,
    })?;

    let _lock = GRAPHVIZ_LOCK.lock().map_err(|e| DotError::SyntaxError {
        message: format!("failed to acquire Graphviz lock: {}", e),
        line: 0,
        column: 0,
    })?;

    unsafe {
        let graph = agmemread(c_dot.as_ptr());
        if graph.is_null() {
            return Err(DotError::SyntaxError {
                message: "failed to parse DOT source".to_string(),
                line: 0,
                column: 0,
            });
        }
        agclose(graph);
        Ok(())
    }
}
