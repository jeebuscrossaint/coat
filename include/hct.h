#ifndef HCT_H
#define HCT_H

// HCT (Hue, Chroma, Tone) color space implementation
// Based on Material Design 3 Color System (CAM16-UCS)

typedef struct {
    double h;  // Hue: 0-360
    double c;  // Chroma: 0-120+
    double t;  // Tone (perceptual lightness): 0-100
} HCT;

typedef struct {
    double x;
    double y;
    double z;
} XYZ;

typedef struct {
    int r;
    int g;
    int b;
} RGB;

typedef struct {
    double h;           // Hue
    double c;           // Chroma
    double j;           // Lightness
    double q;           // Brightness
    double m;           // Colorfulness
    double s;           // Saturation
    double jstar;       // CAM16-UCS J*
    double astar;       // CAM16-UCS a*
    double bstar;       // CAM16-UCS b*
} CAM16;

// Main conversion functions
HCT rgb_to_hct(RGB rgb);
RGB hct_to_rgb(HCT hct);

// Helper conversions
XYZ rgb_to_xyz(RGB rgb);
RGB xyz_to_rgb(XYZ xyz);
CAM16 xyz_to_cam16(XYZ xyz);
XYZ cam16_to_xyz(CAM16 cam);
HCT cam16_to_hct(CAM16 cam);
CAM16 hct_to_cam16(HCT hct);

// Utility functions
int clamp_int(int value, int min, int max);
double clamp_double(double value, double min, double max);

#endif // HCT_H
