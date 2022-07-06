# Datasets

## Download

```bash
wget -O Paid.csv http://prod.publicdata.landregistry.gov.uk.s3-website-eu-west-1.amazonaws.com/pp-complete.csv
mkdir Taxi
cd Taxi
wget https://raw.githubusercontent.com/fivethirtyeight/uber-tlc-foil-response/63bb878b76f47f69b4527d50af57aac26dead983/uber-trip-data/uber-raw-data-apr14.csv
wget https://raw.githubusercontent.com/fivethirtyeight/uber-tlc-foil-response/63bb878b76f47f69b4527d50af57aac26dead983/uber-trip-data/uber-raw-data-aug14.csv
wget https://raw.githubusercontent.com/fivethirtyeight/uber-tlc-foil-response/63bb878b76f47f69b4527d50af57aac26dead983/uber-trip-data/uber-raw-data-jul14.csv
wget https://raw.githubusercontent.com/fivethirtyeight/uber-tlc-foil-response/63bb878b76f47f69b4527d50af57aac26dead983/uber-trip-data/uber-raw-data-jun14.csv
wget https://raw.githubusercontent.com/fivethirtyeight/uber-tlc-foil-response/63bb878b76f47f69b4527d50af57aac26dead983/uber-trip-data/uber-raw-data-may14.csv
wget https://raw.githubusercontent.com/fivethirtyeight/uber-tlc-foil-response/63bb878b76f47f69b4527d50af57aac26dead983/uber-trip-data/uber-raw-data-sep14.csv
mkdir Wiki
cd Wiki
wget https://dumps.wikimedia.org/other/pagecounts-raw/2016/2016-01/pagecounts-20160101-000000.gz
wget https://dumps.wikimedia.org/other/pagecounts-raw/2016/2016-01/pagecounts-20160101-010000.gz
wget https://dumps.wikimedia.org/other/pagecounts-raw/2016/2016-01/pagecounts-20160101-020000.gz
wget https://dumps.wikimedia.org/other/pagecounts-raw/2016/2016-01/pagecounts-20160101-030000.gz
wget https://dumps.wikimedia.org/other/pagecounts-raw/2016/2016-01/pagecounts-20160101-040000.gz
wget https://dumps.wikimedia.org/other/pagecounts-raw/2016/2016-01/pagecounts-20160101-050000.gz
wget https://dumps.wikimedia.org/other/pagecounts-raw/2016/2016-01/pagecounts-20160101-060000.gz
wget https://dumps.wikimedia.org/other/pagecounts-raw/2016/2016-01/pagecounts-20160101-070000.gz
wget https://dumps.wikimedia.org/other/pagecounts-raw/2016/2016-01/pagecounts-20160101-080000.gz
wget https://dumps.wikimedia.org/other/pagecounts-raw/2016/2016-01/pagecounts-20160101-090000.gz
wget https://dumps.wikimedia.org/other/pagecounts-raw/2016/2016-01/pagecounts-20160101-100000.gz
wget https://dumps.wikimedia.org/other/pagecounts-raw/2016/2016-01/pagecounts-20160101-110000.gz
wget https://dumps.wikimedia.org/other/pagecounts-raw/2016/2016-01/pagecounts-20160101-120000.gz
wget https://dumps.wikimedia.org/other/pagecounts-raw/2016/2016-01/pagecounts-20160101-130000.gz
wget https://dumps.wikimedia.org/other/pagecounts-raw/2016/2016-01/pagecounts-20160101-140000.gz
wget https://dumps.wikimedia.org/other/pagecounts-raw/2016/2016-01/pagecounts-20160101-150000.gz
wget https://dumps.wikimedia.org/other/pagecounts-raw/2016/2016-01/pagecounts-20160101-160000.gz
wget https://dumps.wikimedia.org/other/pagecounts-raw/2016/2016-01/pagecounts-20160101-170000.gz
wget https://dumps.wikimedia.org/other/pagecounts-raw/2016/2016-01/pagecounts-20160101-180000.gz
wget https://dumps.wikimedia.org/other/pagecounts-raw/2016/2016-01/pagecounts-20160101-190000.gz
wget https://dumps.wikimedia.org/other/pagecounts-raw/2016/2016-01/pagecounts-20160101-200000.gz
wget https://dumps.wikimedia.org/other/pagecounts-raw/2016/2016-01/pagecounts-20160101-210000.gz
wget https://dumps.wikimedia.org/other/pagecounts-raw/2016/2016-01/pagecounts-20160101-220000.gz
wget https://dumps.wikimedia.org/other/pagecounts-raw/2016/2016-01/pagecounts-20160101-230000.gz
gzip -d ./*
```

[RecipeNLP](https://recipenlg.cs.put.poznan.pl/dataset) should be downloaded manually since there is a Captcha challenge. It should be named "Recipe.csv".
