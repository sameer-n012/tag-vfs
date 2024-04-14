# FileVault

This is a simple file explorer program that can also encrypt/decrypt text files.


To use this program, create a settings.xml file with the following information:

```
<?xml version="1.0" encoding="UTF-8"?>
<root>
        <home_directory>
                HOME_DIRECTORY
        </home_directory>
        <encryption_settings>
                <affine>
                        <slope>1</slope>
                        <intercept>1</intercept>
                        <maintain_case>true</maintain_case>
                </affine>
        </encryption_settings>
        <visual_settings>
                <file_list_box_text_font_size>14</file_list_box_text_font_size>
                <preview_box_sm_text_font_size>20</preview_box_sm_text_font_size>
                <preview_box_lg_text_font_size>20</preview_box_lg_text_font_size>
                <info_box_max_char_length>60</info_box_max_char_length>
                <info_box_text_font_size>14</info_box_text_font_size>
                <action_box_text_font_size>14</action_box_text_font_size>
                <error_scene_text_font_size>14</error_scene_text_font_size>
                <dark_mode>false</dark_mode>
        </visual_settings>
</root>
```

Substitute HOME_DIRECTORY for the directory you want FileVault to open.

Substitute the settings.xml location in src/main/java/com/FileVault/app/Cons.java with the location of your file