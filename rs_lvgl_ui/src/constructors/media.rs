//! Constructors for media views.

use crate::node::{ImageSource, Node, WidgetKind, ANIMATION_REPEAT_INFINITE};
#[cfg(feature = "widget-barcode")]
use crate::Barcode;
#[cfg(feature = "widget-canvas")]
use crate::Canvas;
#[cfg(feature = "widget-gif")]
use crate::Gif;
#[cfg(feature = "widget-lottie")]
use crate::Lottie;
#[cfg(feature = "widget-qrcode")]
use crate::QrCode;
use crate::View;

/// An image at its original scale.
#[cfg(feature = "widget-image")]
pub fn image<M>(source: ImageSource) -> View<M> {
    Node::new(WidgetKind::Image {
        source,
        scale: crate::Scale::percent(100),
    })
}

/// An image with an explicit scale factor; zero is rejected during validation.
#[cfg(feature = "widget-image")]
pub fn image_scaled<M>(source: ImageSource, scale: crate::Scale) -> View<M> {
    Node::new(WidgetKind::Image { source, scale })
}

/// An already loaded image retained through source replacement and unmount.
#[cfg(feature = "widget-image")]
pub fn image_resource<M>(source: crate::ImageResource) -> View<M> {
    image_resource_scaled(source, crate::Scale::percent(100))
}

#[cfg(feature = "widget-image")]
pub fn image_resource_scaled<M>(source: crate::ImageResource, scale: crate::Scale) -> View<M> {
    // SAFETY: the node retains source and transfers it into native retention
    // before its descriptor is discarded, including failure/deletion paths.
    let raw = unsafe { ImageSource::from_raw(source.as_ptr()) };
    let mut node = Node::new(WidgetKind::Image { source: raw, scale });
    node.image_resource = Some(source);
    node
}

/// A button drawn with `released`; add a pressed image with
/// [`View::pressed_image`].
#[cfg(feature = "widget-imagebutton")]
pub fn image_button<M>(released: ImageSource) -> View<M> {
    Node::new(WidgetKind::ImageButton {
        released,
        pressed: None,
    })
}

/// An image that shows every frame once per `duration_ms`, looping
/// forever unless [`View::repeat`] is set.
#[cfg(feature = "widget-animimage")]
pub fn animated_image<M>(
    frames: impl IntoIterator<Item = ImageSource>,
    duration_ms: u32,
) -> View<M> {
    Node::new(WidgetKind::AnimatedImage {
        frames: frames.into_iter().collect(),
        duration_ms,
        repeat: ANIMATION_REPEAT_INFINITE,
    })
}

/// A canvas rendered from an owned frame description.
#[cfg(feature = "widget-canvas")]
pub fn canvas<M>(frame: Canvas) -> View<M> {
    Node::new(WidgetKind::Canvas { frame })
}

/// A QR code with a length-delimited binary payload.
#[cfg(feature = "widget-qrcode")]
pub fn qrcode<M>(code: QrCode) -> View<M> {
    Node::new(WidgetKind::QrCode { code })
}

/// A barcode rendered from its encoding and appearance description.
#[cfg(feature = "widget-barcode")]
pub fn barcode<M>(code: Barcode) -> View<M> {
    let style = crate::Style::default().height(crate::Size::Px(code.height as i16));
    Node::new(WidgetKind::Barcode { code }).style(style)
}

/// A GIF animation with retained encoded bytes.
#[cfg(feature = "widget-gif")]
pub fn gif<M>(animation: Gif) -> View<M> {
    Node::new(WidgetKind::Gif { animation })
}

/// A Lottie animation with retained JSON and a sized drawing buffer.
#[cfg(feature = "widget-lottie")]
pub fn lottie<M>(animation: Lottie) -> View<M> {
    let style = crate::Style::default()
        .width(crate::Size::Px(animation.width as i16))
        .height(crate::Size::Px(animation.height as i16));
    Node::new(WidgetKind::Lottie { animation }).style(style)
}
