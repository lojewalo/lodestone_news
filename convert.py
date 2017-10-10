# This script will convert your lodestone_news.json file to the new sqlite database.
#
# Please run the migrations (diesel migration run) and add any servers to the database first. All
# contents in news_items and send_records will be destroyed.
#
# Files must be named lodestone_news.json and database.sqlite. You can change them in the script if
# renaming is impossible or undesirable.

from json import load
from sqlite3 import connect
from datetime import datetime

def kind(f):
  if f == 'SpecialNotice':
    return 0
  elif f == 'News':
    return 1
  elif f == 'Topic':
    return 2
  else:
    raise Exception("bad kind")

def main():
  input('Run migrations and add servers first! If you\'re ready, hit Enter. If not, Ctrl-C and get ready.')

  with open('lodestone_news.json') as f:
    ln = load(f)

  items = ln['items']
  sorted_items = sorted(items.values(), key = lambda x: x['time'])

  conn = connect('database.sqlite')
  cur = conn.cursor()

  cur.execute('delete from news_items;')

  for item in sorted_items:
    print('Inserting', item['title'])
    params = (item['title'], item['url'], item['url'].split('/')[-1], kind(item['kind']), datetime.fromtimestamp(item['time']), item['tag'])
    cur.execute('insert into news_items (title, url, lodestone_id, kind, created, tag) values (?, ?, ?, ?, ?, ?);', params)

  print('Delete send records')
  cur.execute('delete from send_records;')
  print('Adding all send records')
  cur.execute('insert into send_records (news_id, server_id) select news_items.id, servers.id from news_items, servers;')

  conn.commit()
  conn.close()

if __name__ == '__main__':
  main()
