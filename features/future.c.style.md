# Include MVP Feature

Includes a CSS file from project. The rendered style tag will append the hash of the file to enable cache busting.


Template:
```html
<head>
  <!-- attrs other than href are passed through to output -->
  <Style crossorigin="anonymous" path="src/pages/home/view.css" />
</head>
```

Output:
```html
<head>
  <link crossorigin="anonymous" href="src/pages/home/view.css?5abec1f225ee17f6491b438673e41f3a" />
</head>
```
