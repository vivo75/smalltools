from distutils.core import setup

from setuptools import setup, find_packages
from codecs import open
from os import path

here = path.abspath(path.dirname(__file__))

# Get the long description from the README file
with open(path.join(here, 'README.md'), encoding='utf-8') as f:
    long_description = f.read()

setup(
    name='smalltools',
    version='0.1.2',
    description='Small utilities for sysadmins',
    long_description=long_description,
    url='https://github.com/vivo75/smalltools',
    author='Francesco Riosa',
    author_email='vivo75+smalltools@gmail.com',

    # For a list of valid classifiers, see
    # https://pypi.python.org/pypi?%3Aaction=list_classifiers
    classifiers=[  # Optional
        'Development Status :: 4 - Beta',

        # Indicate who your project is intended for
        'Intended Audience :: Developers',
        'Topic :: System :: Systems Administration',

        # Pick your license as you wish (should match "license" above)
        'License :: OSI Approved :: GNU General Public License v2 or later (GPLv2+)',

        'Programming Language :: Python :: 3.5',
        'Programming Language :: Python :: 3.6',
        'Programming Language :: Python :: 3.7',
    ],

    # Note that this is a string of words separated by whitespace, not a list.
    keywords='management logging helpers',

    packages=['stlog',],
    package_dir = {'': 'src'},

    entry_points={  # Optional
        'console_scripts': [
            'stlog = stlog:main',
        ],
    },
)
