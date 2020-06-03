# Common Issues

## `NoAvailablePixelFormat`

By far, the most common issue I receive every week is about receiving
`NoAvailablePixelFormat`, it is after all a fairly opaque error. 

So what does this error mean? Well, to quote @tomaka:

> Glutin queries the system for the configuration you request (multisampling,
sRGB, depth buffer size, etc.) and if the system doesn't support this
configuration, then `NoAvailablePixelFormat` is returned.

### Debugging on Linux.

On Linux, like on other platforms, debugging this issue is relatively
straightforward. First, we need to know what configs are supported by your
hardware, and what that hardware is. This is trivial, just type the following
into your terminal:

``` 
$ lspci; glxinfo; glinfo; eglinfo
```

Next, track down what attributes glutin is passing to
`eglChooseConfig`/`glXChooseFBConfig`, and compare that to the attributes of the
configs available.

### Debugging on Windows.

Similar to linux, you are going to want to figure out what attributes glutin is
requesting from `ChoosePixelFormatARB`/`ChoosePixelFormat`/`eglChooseConfig`,
however figuring out what to compare them to is less trivial.

The simplest solution I know of is to download the [OpenGL Extension Viewer by
realtech](http://realtech-vr.com/home/glview). You can then go to the "Display
modes & pixel formats" tab and take a look at all the configs supported by WGL,
I believe? I'm not what the equivalent for EGL is, unfortunately.

It should be noted that if you are on Windows and have an AMD gpu, make sure you
are not requesting both a non-sRGB and non-floating point surface, else you
shall be shit out of luck. See:
https://github.com/rust-windowing/glutin/issues/1219
